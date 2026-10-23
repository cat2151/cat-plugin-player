//! Macro rows that move the instrument's own macro parameters through the host's
//! automation path (ADR 0037) instead of CCs.
use crate::config::PluginKey;
use crate::ffi::{
    self,
    parameters::{domain::Position, ParameterValue},
};
use crate::sequence_modulation::MACROS;

/// One macro parameter's binding and its value at each CC-scale level 0-127.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Binding {
    parameter: ParameterValue,
    values: [f64; 128],
}

/// Per macro; `None` where the instrument has no such parameter.
pub(crate) type Bindings = [Option<Binding>; MACROS];

/// Prepares `processor` and binds macro N to the eligible parameter the instrument names
/// for it. Call with every audio renderer stopped. CC-driven instruments get no bindings.
pub(crate) fn bind(
    host: &ffi::Host,
    processor: &ffi::Processor,
    id: i32,
    plugin: &PluginKey,
) -> Result<Bindings, String> {
    let mut bindings = [None; MACROS];
    let Some(instrument) = crate::plugin_specific::parameter_macros(plugin) else {
        return Ok(bindings);
    };
    processor.prepare_parameters()?;
    let parameters = host.parameters(id)?;
    for (index, binding) in bindings.iter_mut().enumerate() {
        let Some(parameter) = parameters
            .iter()
            .find(|p| p.eligible && instrument.parameter(index + 1, &p.name))
        else {
            continue;
        };
        let domain = parameter.domain()?;
        let mut values = [0.0; 128];
        for (level, value) in values.iter_mut().enumerate() {
            *value = domain.api(Position::new(level as f64 / 127.0)?).get();
        }
        *binding = Some(Binding {
            parameter: parameter.binding,
            values,
        });
    }
    Ok(bindings)
}

/// Hands the bound macros' `levels` to the plugin with the next block; call right before
/// processing it. Audio thread: no allocation or lock. `Err` carries the shim's code.
pub(crate) fn send(
    render: &mut ffi::ProcessorRef,
    bindings: &Bindings,
    levels: [Option<u8>; MACROS],
    scratch: &mut [ParameterValue; MACROS],
) -> Result<(), i32> {
    match batch(bindings, levels, scratch) {
        0 => Ok(()),
        count => render.parameters(&scratch[..count]),
    }
}

/// Fills `out` with the values `levels` give the bound macros and returns how many.
fn batch(
    bindings: &Bindings,
    levels: [Option<u8>; MACROS],
    out: &mut [ParameterValue; MACROS],
) -> usize {
    let mut count = 0;
    for (binding, level) in bindings.iter().zip(levels) {
        if let (Some(binding), Some(level)) = (binding, level) {
            out[count] = binding
                .parameter
                .with_value(binding.values[usize::from(level.min(127))]);
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_map_linearly_onto_bound_parameters_only() {
        let bipolar = Binding {
            parameter: ParameterValue::default(),
            values: std::array::from_fn(|level| -1.0 + 2.0 * level as f64 / 127.0),
        };
        let mut bindings = [None; MACROS];
        bindings[0] = Some(bipolar);
        bindings[3] = Some(bipolar);
        let mut levels = [None; MACROS];
        levels[0] = Some(127);
        levels[1] = Some(64);
        levels[3] = Some(0);
        let mut out = [ParameterValue::default(); MACROS];
        assert_eq!(batch(&bindings, levels, &mut out), 2);
        assert_eq!((out[0].value(), out[1].value()), (1.0, -1.0));
        assert_eq!(batch(&bindings, [None; MACROS], &mut out), 0);
    }
}
