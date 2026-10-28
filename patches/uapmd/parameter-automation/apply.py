"""Compose build-local parameter sources after existing CLAP note/state repairs."""
from pathlib import Path
import hashlib
import re
import subprocess
import sys
import importlib.util
import tempfile

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
BASELINES = {
    'vst3/PluginInstanceVST3.States.cpp': '899c53d931a687115ed526f197c94e8564ee91e826c1c1dcab879205868c68e2',
    'vst3/HostClasses.hpp': 'ceaae1d61b89f4f74597a865e10d89b6a48f2a683868c8b8f9efde9f1499b81e',
    'clap/PluginInstanceCLAP.Parameters.cpp': 'cf7b01ee3867baecf2de9fa71060c165a7d98f87000de682fc1e433e78a7be85',
    'vst3/PluginFormatVST3.hpp': '476e8e208f510f762e90a072f47c72742a82a1ee08a7adf0f6452a241646cc9a',
    'vst3/PluginInstanceVST3.cpp': 'e42d362ec2c9b97ebfdb49ae207e30b827776c4e00ed814549cfabfbb803673e',
    'vst3/PluginInstanceVST3.Parameters.cpp': 'ad438ffa987b157d19b2eb8ff845d7494915acf1908b63abd2946c598b19d6f9',
}


def load_patch(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), HERE.parent / name / 'apply.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_composition(source, note, state, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='parameter-validation-', dir=output.parent) as temp:
        expected_note, expected_state = Path(temp) / 'note', Path(temp) / 'state'
        load_patch('clap-note-dialect').generate(source / 'clap', expected_note)
        load_patch('clap-state-load-error').generate(source / 'clap', expected_note, expected_state)
        for directory, actual in [(expected_note, note), (expected_state, state)]:
            for p in directory.iterdir():
                expected = p.read_text(encoding='utf-8').replace(expected_note.as_posix(), note.as_posix())
                if expected != (actual / p.name).read_text(encoding='utf-8'):
                    raise ValueError(f'unexpected composed input: {actual / p.name}')


def generate(source, note, state, output):
    source, note, state, output = [Path(p).resolve() for p in (source, note, state, output)]
    current = source
    try:
        if any(output == p or p in output.parents for p in (source, note, state)):
            raise ValueError('output must be outside all input directories')
        for name, sha in BASELINES.items():
            current = source / name
            if hashlib.sha256(current.read_bytes().replace(b'\r\n', b'\n')).hexdigest() != sha:
                raise ValueError('baseline SHA256 mismatch; inspect upstream change/fix')
        current = note
        validate_composition(source, note, state, output)
        texts = {}
        for p in note.glob('*'):
            if p.suffix in ('.cpp', '.hpp'):
                texts['clap/' + p.name] = p.read_text(encoding='utf-8')
        texts['clap/PluginInstanceCLAP.States.cpp'] = (state / 'PluginInstanceCLAP.States.cpp').read_text(encoding='utf-8')
        for p in (source / 'vst3').glob('*'):
            if p.suffix in ('.cpp', '.hpp'):
                texts['vst3/' + p.name] = p.read_text(encoding='utf-8')

        def once(name, old, new):
            nonlocal current
            current = source / name
            if texts[name].count(old) != 1:
                raise ValueError(f'expected exactly one anchor: {old!r}')
            texts[name] = texts[name].replace(old, new)

        ch = 'clap/PluginFormatCLAP.hpp'
        cp = 'clap/PluginInstanceCLAP.Parameters.cpp'
        texts[ch] = f'#include "{(HERE / "backend_api.h").as_posix()}"\n' + texts[ch]
        once(ch, '        class ParameterSupport : public PluginParameterSupport {',
             '''        bool automation_enabled{false}, automation_output_overflow{false};
        static bool automationPush(const clap_output_events_t* list, const clap_event_header_t* event) {
            auto* self = static_cast<PluginInstanceCLAP*>(list->ctx);
            bool ok = self->events_out->tryPush(event);
            self->automation_output_overflow |= !ok; return ok;
        }
        clap_output_events_t automation_output{this, automationPush};
    public:
        class ParameterSupport : public PluginParameterSupport {''')
        once(ch, '        class CLAPUmpInputDispatcher : public TypedUmpInputDispatcher {',
             '    private:\n        class CLAPUmpInputDispatcher : public TypedUmpInputDispatcher {')
        once(ch, '            explicit ParameterSupport(PluginInstanceCLAP* owner);', '''            bool resolveAutomation(uint32_t index, uh::automation::Target* target) {
                if (!target || index >= parameter_ids.size()) return false;
                *target = {parameter_ids[index], parameter_cookies[index]}; return true;
            }
            StatusCode applyAutomation(uh::automation::Target target, double value);
            StatusCode noteAutomation(uint32_t word);
            void prepareAutomation();
            void cancelAutomation();
            explicit ParameterSupport(PluginInstanceCLAP* owner);''')
        once(cp, '''                        true,
                        info.flags & CLAP_PARAM_IS_AUTOMATABLE,''', '''                        (info.flags & CLAP_PARAM_IS_AUTOMATABLE) && !(info.flags & CLAP_PARAM_IS_READONLY),
                        true,''')
        # Metadata indices address the successfully enumerated vectors, not skipped native entries.
        once(cp, '                        static_cast<uint32_t>(i),', '                        static_cast<uint32_t>(parameter_defs.size()),')
        once(cp, 'param_id_to_index[info.id] = static_cast<uint32_t>(i);',
             'param_id_to_index[info.id] = static_cast<uint32_t>(parameter_defs.size() - 1);')
        start = texts[cp].index('    StatusCode PluginInstanceCLAP::ParameterSupport::enqueueParameterRT(')
        end = texts[cp].index('    StatusCode PluginInstanceCLAP::ParameterSupport::setPerNoteController(', start)
        method = texts[cp][start:end].replace('evt->header.flags = CLAP_EVENT_IS_LIVE;', 'evt->header.flags = 0;')
        method = method.replace('evt->port_index = 0;', 'evt->note_id = -1;\n        evt->key = -1;\n        evt->port_index = -1;').replace('evt->channel = 0;', 'evt->channel = -1;')
        texts[cp] = texts[cp][:start] + method + texts[cp][end:]
        texts[cp] += '''
remidy::StatusCode remidy::PluginInstanceCLAP::ParameterSupport::noteAutomation(uint32_t word) {
    auto type=(word>>20)&15;
    if (type==11 || type==14) {
        if (!(owner->inputNoteDialects(0) & CLAP_NOTE_DIALECT_MIDI)) return StatusCode::INVALID_PARAMETER_OPERATION;
        auto* event=reinterpret_cast<clap_event_midi_t*>(owner->events_in->tryAllocate(alignof(void*),sizeof(clap_event_midi_t)));
        if (!event) return StatusCode::INSUFFICIENT_MEMORY;
        *event={};event->header={sizeof(clap_event_midi_t),0,CLAP_CORE_EVENT_SPACE_ID,CLAP_EVENT_MIDI,0};
        event->port_index=0;event->data[0]=(word>>16)&255;event->data[1]=(word>>8)&127;event->data[2]=word&127;
        return StatusCode::OK;
    }
    const auto before=owner->events_in->size();
    const auto velocity=static_cast<uint16_t>((word & 127) * 65535 / 127);
    uh::clap_dialect::note([&](size_t size)->void* { return owner->events_in->tryAllocate(alignof(void*),size); },
        owner->inputNoteDialects(0), ((word>>20)&15)==9 && velocity, 0, (word>>16)&15, (word>>8)&127,velocity,0);
    return owner->events_in->size()==before+1 ? StatusCode::OK : StatusCode::INSUFFICIENT_MEMORY;
}
remidy::StatusCode remidy::PluginInstanceCLAP::ParameterSupport::applyAutomation(uh::automation::Target target, double value) {
    auto* evt = reinterpret_cast<clap_event_param_value_t*>(owner->events_in->tryAllocate(alignof(void*), sizeof(clap_event_param_value_t)));
    if (!evt) return StatusCode::INSUFFICIENT_MEMORY;
    *evt = {};
    evt->header = {sizeof(clap_event_param_value_t), 0, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_PARAM_VALUE, 0};
    evt->note_id = -1; evt->port_index = -1; evt->channel = -1; evt->key = -1;
    evt->param_id = target.id; evt->cookie = target.cookie; evt->value = value;
    return StatusCode::OK;
}
void remidy::PluginInstanceCLAP::ParameterSupport::prepareAutomation() {
    parameters(); // UI thread only; never enumerate from enqueue.
    owner->events_in->reserveEvents(16384);
    owner->events_in->reserveHeap(1024 * 1024);
    owner->automation_enabled = true;
}
void remidy::PluginInstanceCLAP::ParameterSupport::cancelAutomation() { owner->events_in->clear(); }
namespace uh::automation {
int clapNote(remidy::PluginParameterSupport* p, uint32_t word) {
    auto* s=dynamic_cast<remidy::PluginInstanceCLAP::ParameterSupport*>(p);
    return s && s->noteAutomation(word)==remidy::StatusCode::OK ? 0 : -1;
}
bool clapResolve(remidy::PluginParameterSupport* p, uint32_t index, Target* target) {
    auto* s = dynamic_cast<remidy::PluginInstanceCLAP::ParameterSupport*>(p);
    return s && s->resolveAutomation(index, target);
}
int clapApply(remidy::PluginParameterSupport* p, Target target, double value) {
    auto* s = dynamic_cast<remidy::PluginInstanceCLAP::ParameterSupport*>(p);
    return s && s->applyAutomation(target, value) == remidy::StatusCode::OK ? 0 : -1;
}
bool clapPrepare(remidy::PluginParameterSupport* p) {
    auto* s = dynamic_cast<remidy::PluginInstanceCLAP::ParameterSupport*>(p);
    if (!s) return false; s->prepareAutomation(); return true;
}
void clapCancel(remidy::PluginParameterSupport* p) {
    if (auto* s = dynamic_cast<remidy::PluginInstanceCLAP::ParameterSupport*>(p)) s->cancelAutomation();
}
}
'''
        # Output events still become UMP; avoid the mutex/listener path for automation owners.
        cc = 'clap/PluginInstanceCLAP.cpp'
        once(cc, '                    if (_parameters) {',
             '                    if (_parameters && !automation_enabled) {')
        once(cc, 'clap_process.out_events = events_out->clapOutputEvents();',
             'clap_process.out_events = automation_enabled ? &automation_output : events_out->clapOutputEvents();')
        once(cc, '        auto result = plugin->process(&clap_process);',
             '        automation_output_overflow = false;\n        auto result = plugin->process(&clap_process);')
        once(cc, 'auto ret = result == CLAP_PROCESS_ERROR ? StatusCode::FAILED_TO_PROCESS : StatusCode::OK;',
             'auto ret = automation_output_overflow ? StatusCode::INSUFFICIENT_MEMORY : result == CLAP_PROCESS_ERROR ? StatusCode::FAILED_TO_PROCESS : StatusCode::OK;')

        vh = 'vst3/PluginFormatVST3.hpp'
        vp = 'vst3/PluginInstanceVST3.Parameters.cpp'
        vc = 'vst3/PluginInstanceVST3.cpp'
        once('vst3/PluginInstanceVST3.States.cpp','        bool resumeProcessing = false;',
             '        owner->automation_current_valid.store(false, std::memory_order_release);\n        bool resumeProcessing = false;')
        host = 'vst3/HostClasses.hpp'
        once(host, '        std::vector<Event> events{};', '''        std::vector<Event> events{};
        bool bounded{}, failed{};''')
        once(host, '        explicit HostEventList() = default;', '''        explicit HostEventList() = default;
        void prepareBounded(size_t capacity) { events.reserve(capacity); bounded = true; }
        bool overflowed() const { return failed; }''')
        once(host, '            events.emplace_back(e);', '''            if (bounded && events.size() == events.capacity()) { failed = true; return kOutOfMemory; }
            events.emplace_back(e);''')
        once(host, '            events.clear();', '            events.clear(); failed = false;')
        texts[vh] = f'#include "{(HERE / "backend_api.h").as_posix()}"\n' + texts[vh]
        once(vh, '        class ParameterSupport : public PluginParameterSupport {',
             '    public:\n        class ParameterSupport : public PluginParameterSupport {')
        once(vh, '        class VST3UmpInputDispatcher : public TypedUmpInputDispatcher {',
             '    private:\n        class VST3UmpInputDispatcher : public TypedUmpInputDispatcher {')
        texts[vh] = f'#include "{(HERE / "fixed_changes.h").as_posix()}"\n' + texts[vh]
        once(vh, '        HostParameterChanges processDataInputParameterChanges{};', '''        bool automation_enabled{false};
        std::unique_ptr<std::atomic<double>[]> automation_current;
        size_t automation_current_count{};
        std::atomic<bool> automation_current_valid{false};
        struct CurrentChange { std::atomic<double>* target{}; double value{}; };
        std::array<CurrentChange,512> automation_current_pending{};
        size_t automation_current_pending_count{};
        uh::automation::FixedChanges automation_changes{}, automation_output_changes{};
        HostEventList automation_note_events{};
        std::array<std::array<uh::automation::Target,129>,16> automation_midi_targets{};
        std::array<std::array<bool,129>,16> automation_midi_mapped{};
        HostParameterChanges processDataInputParameterChanges{};''')
        once(vh, '            explicit ParameterSupport(PluginInstanceVST3* owner);', '''            bool resolveAutomation(uint32_t index, uh::automation::Target* target) {
                if (!target || index >= parameter_ids.size()) return false;
                if (index >= owner->automation_current_count) return false;
                *target = {parameter_ids[index], nullptr, &owner->automation_current[index]}; return true;
            }
            StatusCode applyAutomation(uh::automation::Target target, double value);
            StatusCode noteAutomation(uint32_t word);
            void prepareAutomation() {
                owner->automation_current_pending_count=0;
                static_assert(std::atomic<double>::is_always_lock_free);
                owner->automation_current = std::make_unique<std::atomic<double>[]>(parameter_ids.size());
                owner->automation_current_count = parameter_ids.size();
                for (size_t i = 0; i < parameter_ids.size(); ++i)
                    owner->automation_current[i].store(owner->controller->getParamNormalized(parameter_ids[i]), std::memory_order_relaxed);
                owner->automation_current_valid.store(true, std::memory_order_release);
                for (auto& row: owner->automation_midi_mapped) row.fill(false);
                auto cache=[&](const auto& assignments) {
                    for (const auto& a: assignments) {
                        if (a.controller>=128 && a.controller!=ControllerNumbers::kPitchBend) continue;
                        auto cc=a.controller==ControllerNumbers::kPitchBend ? 128 : int(a.controller);
                        if (a.busIndex!=0 || a.channel<0 || a.channel>=16 || cc<0 || cc>128) continue;
                        uh::automation::Target target{a.pId,nullptr,nullptr};
                        auto index=param_id_to_index.find(a.pId);
                        if (index!=param_id_to_index.end() && index->second<owner->automation_current_count)
                            target.current=&owner->automation_current[index->second];
                        owner->automation_midi_targets[a.channel][cc]=target;
                        owner->automation_midi_mapped[a.channel][cc]=true;
                    }
                };
                cache(owner->cached_midi1_mappings_from_mapping);
                cache(owner->cached_midi1_mappings_from_mapping2);
                owner->processDataInputEvents.prepareBounded(16384);
                owner->automation_note_events.prepareBounded(16384);
                owner->processDataOutputEvents.prepareBounded(16384);
                owner->processData.outputParameterChanges = &owner->automation_output_changes;
                owner->automation_enabled = true;
            }
            void cancelAutomation() { owner->automation_changes.clear(); owner->automation_note_events.clear(); owner->automation_current_pending_count=0; }
            StatusCode currentAutomation(uint32_t index, double* value);
            void recordAutomation(ParamID id, double value) {
                if (!owner->automation_current_valid.load(std::memory_order_acquire)) return;
                auto it = param_id_to_index.find(id);
                if (it != param_id_to_index.end() && it->second < owner->automation_current_count)
                    owner->automation_current[it->second].store(value, std::memory_order_relaxed);
            }
            std::string textAutomation(uint32_t index, double value);
            explicit ParameterSupport(PluginInstanceVST3* owner);''')
        once(vp, 'info.stepCount > 0 || (info.flags & ParameterInfo::kCanAutomate),',
             '(info.flags & ParameterInfo::kCanAutomate) && !(info.flags & (ParameterInfo::kIsReadOnly | ParameterInfo::kIsProgramChange)),')
        once(vp, 'info.flags & ParameterInfo::kIsList,', 'info.stepCount > 0 || (info.flags & ParameterInfo::kIsList),')
        # Leave legacy plain enqueue alone; automation uses its own normalized bounded queue.
        texts[vp] += '''
remidy::StatusCode remidy::PluginInstanceVST3::ParameterSupport::noteAutomation(uint32_t word) {
    auto type=(word>>20)&15;
    if (type==11 || type==14) {
        auto channel=(word>>16)&15;auto cc=type==14 ? 128 : ((word>>8)&127);
        if (!owner->automation_midi_mapped[channel][cc]) return StatusCode::INVALID_PARAMETER_OPERATION;
        double value=type==14 ? (((word&127)<<7)|((word>>8)&127))/16383.0 : (word&127)/127.0;
        return applyAutomation(owner->automation_midi_targets[channel][cc],value);
    }
    Event event{}; event.busIndex=0; event.sampleOffset=0;
    bool on=((word>>20)&15)==9 && (word&127);
    event.type=on ? Event::kNoteOnEvent : Event::kNoteOffEvent;
    if (on) {event.noteOn.channel=(word>>16)&15;event.noteOn.pitch=(word>>8)&127;event.noteOn.velocity=(word&127)/127.f;event.noteOn.noteId=-1;}
    else {event.noteOff.channel=(word>>16)&15;event.noteOff.pitch=(word>>8)&127;event.noteOff.velocity=(word&127)/127.f;event.noteOff.noteId=-1;}
    return owner->automation_note_events.addEvent(event)==kResultOk ? StatusCode::OK : StatusCode::INSUFFICIENT_MEMORY;
}
remidy::StatusCode remidy::PluginInstanceVST3::ParameterSupport::applyAutomation(uh::automation::Target target, double value) {
    if (!owner->automation_enabled || !owner->automation_current_valid.load(std::memory_order_acquire)) return StatusCode::INVALID_PARAMETER_OPERATION;
    if (owner->automation_current_pending_count==owner->automation_current_pending.size()) return StatusCode::INSUFFICIENT_MEMORY;
    int32 i; auto* q = owner->automation_changes.addParameterData(target.id, i);
    if (!q || q->addPoint(0, value, i) != kResultOk) return StatusCode::INSUFFICIENT_MEMORY;
    owner->automation_current_pending[owner->automation_current_pending_count++]={target.current,value};
    return StatusCode::OK;
}
remidy::StatusCode remidy::PluginInstanceVST3::ParameterSupport::currentAutomation(uint32_t index, double* value) {
    if (!value || index >= parameter_ids.size() || !owner->controller) return StatusCode::INVALID_PARAMETER_OPERATION;
    *value = owner->automation_current_valid.load(std::memory_order_acquire) && index < owner->automation_current_count
        ? owner->automation_current[index].load(std::memory_order_relaxed)
        : owner->controller->getParamNormalized(parameter_ids[index]);
    return StatusCode::OK;
}
std::string remidy::PluginInstanceVST3::ParameterSupport::textAutomation(uint32_t index, double value) {
    if (index >= parameter_ids.size() || !owner->controller) return {};
    String128 text{};
    return owner->controller->getParamStringByValue(parameter_ids[index], value, text) == kResultOk ? vst3StringToStdString(text) : "";
}
namespace uh::automation {
int vst3Note(remidy::PluginParameterSupport* p,uint32_t word) {
    auto* s=dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    return s && s->noteAutomation(word)==remidy::StatusCode::OK ? 0 : -1;
}
bool vst3Resolve(remidy::PluginParameterSupport* p, uint32_t index, Target* target) {
    auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    return s && s->resolveAutomation(index, target);
}
int vst3Apply(remidy::PluginParameterSupport* p, Target target, double value) {
    auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    return s && s->applyAutomation(target, value) == remidy::StatusCode::OK ? 0 : -1;
}
bool vst3Prepare(remidy::PluginParameterSupport* p) {
    auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    if (!s) return false; s->prepareAutomation(); return true;
}
void vst3Cancel(remidy::PluginParameterSupport* p) {
    if (auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p)) s->cancelAutomation();
}
int vst3Current(remidy::PluginParameterSupport* p, uint32_t index, double* value) {
    auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    return s && s->currentAutomation(index, value) == remidy::StatusCode::OK ? 0 : -1;
}
std::string vst3Text(remidy::PluginParameterSupport* p, uint32_t index, double value) {
    auto* s = dynamic_cast<remidy::PluginInstanceVST3::ParameterSupport*>(p);
    return s ? s->textAutomation(index, value) : "";
}
}
'''
        once(vc, '    auto result = processor->process(processData);', '''    if (automation_enabled) {
        if (processDataInputEvents.overflowed() || !automation_changes.append(processDataInputParameterChanges)) {
            automation_changes.clear(); automation_current_pending_count=0; processDataInputParameterChanges.clear();
            return StatusCode::INSUFFICIENT_MEMORY;
        }
        automation_output_changes.clear();
        processData.inputParameterChanges = &automation_changes;
    }
    auto result = processor->process(processData);
    processData.inputParameterChanges = processDataInputParameterChanges.asInterface();
    automation_changes.clear();
    if (automation_enabled) {
        if (result==kResultOk && !processDataOutputEvents.overflowed() && !automation_output_changes.overflowed()
            && automation_current_valid.load(std::memory_order_acquire))
            for (size_t i=0;i<automation_current_pending_count;++i)
                if (automation_current_pending[i].target)
                    automation_current_pending[i].target->store(automation_current_pending[i].value,std::memory_order_relaxed);
        automation_current_pending_count=0;
    }''')
        once(vc,'    processDataInputEvents.clear();\n    ump_input_dispatcher.process(process);', '''    processDataInputEvents.clear();
    if (automation_enabled) {
        for (int32 i=0;i<automation_note_events.getEventCount();++i) {
            Event event{}; automation_note_events.getEvent(i,event);
            event.ppqPosition=process_context.projectTimeMusic;
            if (processDataInputEvents.addEvent(event)!=kResultOk) {automation_note_events.clear();return StatusCode::INSUFFICIENT_MEMORY;}
        }
        automation_note_events.clear();
    }
    ump_input_dispatcher.process(process);''')
        once(vp, 'void remidy::PluginInstanceVST3::ParameterSupport::clearParameterList() {',
             'void remidy::PluginInstanceVST3::ParameterSupport::clearParameterList() {\n    owner->automation_current_valid.store(false, std::memory_order_release);')
        once(vc, 'double plainValue = controller->normalizedParamToPlain(paramId, value);\n        dynamic_cast<ParameterSupport*>(parameters())->notifyParameterValue(paramId, plainValue);',
             'dynamic_cast<ParameterSupport*>(parameters())->recordAutomation(paramId, value);\n        double plainValue = controller->normalizedParamToPlain(paramId, value);\n        dynamic_cast<ParameterSupport*>(parameters())->notifyParameterValue(paramId, plainValue);')
        once(vc, '    if (result != kResultOk) {\n        owner->getLogger()->logError("Failed to process vst3 audio. Result: %d", result);', '''    if (automation_enabled && (processDataOutputEvents.overflowed() || automation_output_changes.overflowed()))
        return StatusCode::INSUFFICIENT_MEMORY;
    if (result != kResultOk) {
        owner->getLogger()->logError("Failed to process vst3 audio. Result: %d", result);''')
        once(vc, 'int32_t paramCount = processDataOutputParameterChanges.getParameterCount();',
             'int32_t paramCount = processData.outputParameterChanges->getParameterCount();')
        once(vc, 'auto* queue = processDataOutputParameterChanges.getParameterData(i);',
             'auto* queue = processData.outputParameterChanges->getParameterData(i);')
        once(vc, 'double plainValue = controller->normalizedParamToPlain(paramId, value);\n                    dynamic_cast<PluginInstanceVST3::ParameterSupport*>(parameters())->notifyParameterValue(paramId, plainValue);',
             '''if (!automation_enabled) {
                        double plainValue = controller->normalizedParamToPlain(paramId, value);
                        dynamic_cast<PluginInstanceVST3::ParameterSupport*>(parameters())->notifyParameterValue(paramId, plainValue);
                    }''')

        # Every backend TU uses one class definition, including existing state/note repairs.
        for name, text in texts.items():
            kind = name.split('/')[0]
            def include(match):
                path = Path(match[1])
                if path.name == ('PluginFormatCLAP.hpp' if kind == 'clap' else 'PluginFormatVST3.hpp'):
                    return f'#include "{(output / kind / path.name).as_posix()}"'
                if kind == 'vst3' and path.name == 'HostClasses.hpp':
                    return f'#include "{(output / kind / path.name).as_posix()}"'
                original = source / kind / path
                if not path.is_absolute() and original.is_file():
                    return f'#include "{original.resolve().as_posix()}"'
                return match[0]
            text = re.sub(r'#include "([^"\n]+)"', include, text)
            dest = output / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            if not dest.exists() or dest.read_text(encoding='utf-8') != text:
                dest.write_text(text, encoding='utf-8', newline='\n')
        return sorted(name for name in texts if name.endswith('.cpp'))
    except (OSError, ValueError) as error:
        revision = subprocess.run(['git', '-C', str(source), 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip() or 'unknown'
        raise RuntimeError(f'[parameter-automation] NOT applied; target: {current}; condition: {error}; '
                           f'UAPMD revision: {revision}; instructions: {HERE / "README.md"}') from error


if __name__ == '__main__':
    try:
        print(';'.join(generate(*sys.argv[1:])))
    except (RuntimeError, TypeError) as error:
        sys.exit(str(error))
