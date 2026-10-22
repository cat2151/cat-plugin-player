// Included after the shim's private types/helpers. ABI definitions remain here
// so parameter discovery never changes the normal processor construction path.
namespace {
UhProcessor* parameterStage(UhProcessor* p) {
    if (!p || p->effects_only) return nullptr;
    return p->stages.empty() ? p : p->stages.front().get();
}
bool parameterFormat(uapmd_plugin_hosting::AudioPluginInstanceAPI* instance) {
    return instance && (instance->formatName() == "CLAP" || instance->formatName() == "VST3");
}
template<size_t N> bool parameterString(char (&out)[N], const std::string& text) {
    if (text.size() >= N) return false;
    std::memcpy(out, text.c_str(), text.size() + 1); return true;
}
}
extern "C" {
int32_t uh_parameter_count(UhHost* host, int32_t id) {
    if (!host) return -1;
    try {
        auto* instance = getInstance(host, id);
        if (!parameterFormat(instance)) return -2;
        auto* support = instance->parameterSupport();
        return support ? static_cast<int32_t>(support->parameters().size()) : -2;
    } catch (...) { return UH_ERR_EXCEPTION; }
}
int32_t uh_parameter_info(UhHost* host, int32_t id, uint32_t index, UhParameterInfo* out) {
    if (!host || !out) return -1;
    *out = {};
    try {
        auto it = host->instances.find(id);
        if (it == host->instances.end()) return -1;
        auto* instance = it->second->lifecycle->instance();
        if (!parameterFormat(instance)) return -2;
        auto* support = instance->parameterSupport();
        if (!support) return -2;
        auto& list = support->parameters();
        if (index >= list.size() || !list[index]) return -3;
        auto* p = list[index];
        out->generation = it->second->parameter_generation;
        out->revision = it->second->parameter_list_changes.load(std::memory_order_acquire);
        out->index = p->index(); out->normalized = instance->formatName() == "VST3";
        out->minimum = out->normalized ? 0.0 : p->minPlainValue();
        out->maximum = out->normalized ? 1.0 : p->maxPlainValue();
        out->eligible = p->automatable() && p->readable() && !p->hidden() && !p->discrete()
            && std::isfinite(out->minimum) && std::isfinite(out->maximum) && out->minimum < out->maximum;
        int result = out->normalized ? uh::automation::vst3Current(support, p->index(), &out->current)
            : (support->getParameter(p->index(), &out->current) == remidy::StatusCode::OK ? 0 : -1);
        if (result || !std::isfinite(out->current)) return -4;
        return parameterString(out->stable_id, p->stableId()) && parameterString(out->name, p->name())
            && parameterString(out->path, p->path()) ? 0 : -7;
    } catch (...) { return UH_ERR_EXCEPTION; }
}
int32_t uh_parameter_text(UhHost* host, int32_t id, const UhParameterValue* value, char* out, int32_t size) {
    if (!host || !value || !out || size <= 0) return -1;
    out[0] = 0;
    try {
        auto it = host->instances.find(id);
        if (it == host->instances.end() || value->generation != it->second->parameter_generation
            || value->revision != it->second->parameter_list_changes.load(std::memory_order_acquire)) return -6;
        auto* instance = it->second->lifecycle->instance();
        if (!parameterFormat(instance) || !std::isfinite(value->value)) return -2;
        auto* support = instance->parameterSupport();
        if (!support || value->index >= support->parameters().size()) return -3;
        auto text = instance->formatName() == "VST3" ? uh::automation::vst3Text(support, value->index, value->value)
            : support->valueToString(value->index, value->value);
        if (text.size() >= static_cast<size_t>(size)) return -7;
        std::memcpy(out, text.c_str(), text.size() + 1); return 0;
    } catch (...) { return UH_ERR_EXCEPTION; }
}
int32_t uh_processor_prepare_parameters(UhProcessor* processor) {
    auto* p = parameterStage(processor);
    if (!p || !p->parameters.owned || !parameterFormat(p->instance)) return -1;
    try {
        auto& b = p->parameters;
        b.prepared = false; b.pending = false;
        b.support = p->instance->parameterSupport();
        if (!b.support) return -2;
        b.normalized = p->instance->formatName() == "VST3";
        if (!(b.normalized ? uh::automation::vst3Prepare(b.support) : uh::automation::clapPrepare(b.support))) return -2;
        b.enqueue = b.normalized ? uh::automation::vst3Apply : uh::automation::clapApply;
        b.note = b.normalized ? uh::automation::vst3Note : uh::automation::clapNote;
        b.cancel = b.normalized ? uh::automation::vst3Cancel : uh::automation::clapCancel;
        b.definitions.clear();
        for (auto* item : b.support->parameters()) {
            if (item->index() != b.definitions.size()) return -3;
            double min = b.normalized ? 0.0 : item->minPlainValue();
            double max = b.normalized ? 1.0 : item->maxPlainValue();
            uh::automation::Target target;
            if (!(b.normalized ? uh::automation::vst3Resolve(b.support, item->index(), &target)
                              : uh::automation::clapResolve(b.support, item->index(), &target))) return -3;
            b.definitions.push_back({min, max, item->automatable() && item->readable()
                && !item->hidden() && !item->discrete() && std::isfinite(min) && std::isfinite(max) && min < max, target});
        }
        b.generation = b.owned->parameter_generation;
        b.revision = b.owned->parameter_list_changes.load(std::memory_order_acquire);
        b.prepared = true; return 0;
    } catch (...) { return UH_ERR_EXCEPTION; }
}
int32_t uh_processor_parameter_batch(UhProcessor* processor, const UhParameterValue* values, uint32_t count) {
    auto* p = parameterStage(processor);
    if (!p || (count && !values)) return -1;
    try {
    auto& b = p->parameters;
    if (!b.valid() || b.pending) { b.clear(); return -6; }
    if (count > 256) return -7;
    for (uint32_t i = 0; i < count; ++i) {
        const auto& v = values[i];
        if (v.generation != b.generation || v.revision != b.revision || v.index >= b.definitions.size()) return -6;
        const auto& d = b.definitions[v.index];
        if (!d.eligible || !std::isfinite(v.value) || v.value < d.min || v.value > d.max) return -3;
        for (uint32_t j = 0; j < i; ++j) if (values[j].index == v.index) return -3;
    }
    for (uint32_t i = 0; i < count; ++i)
        if (b.enqueue(b.support, b.definitions[values[i].index].target, values[i].value) != 0) { b.clear(); return -7; }
    b.pending = count != 0; return 0;
    } catch (...) {p->parameters.clear();return UH_ERR_EXCEPTION;}
}

int32_t uh_processor_process_automation(UhProcessor* processor,const UhAutomationEvent* events,uint32_t count,
                                       float* out,int32_t channels,int32_t frames) {
    auto* p=parameterStage(processor);
    if (!p || !out || channels<=0 || frames<=0 || (count && !events)) return -1;
    try {
    auto& b=p->parameters;
    if (!b.valid() || b.pending) {b.clear();return -6;}
    if (count>512) return -7;
    // Validate the entire ordered batch before touching either native queue.
    for (uint32_t i=0;i<count;++i) {
        const auto& e=events[i];
        if (e.kind==0) {
            const auto& v=e.parameter;
            if (v.generation!=b.generation || v.revision!=b.revision || v.index>=b.definitions.size()) return -6;
            const auto& d=b.definitions[v.index];
            if (!d.eligible || !std::isfinite(v.value) || v.value<d.min || v.value>d.max) return -3;
        } else if (e.kind==1) {
            auto type=(e.midi>>20)&15;
            if ((e.midi>>28)!=2 || (e.midi&0x0f000000) || (type!=8 && type!=9) || (e.midi&0x8080)) return -3;
        } else return -3;
    }
    for (uint32_t i=0;i<count;++i) {
        const auto& e=events[i];
        int code=e.kind==0 ? b.enqueue(b.support,b.definitions[e.parameter.index].target,e.parameter.value)
                           : b.note(b.support,e.midi);
        if (code) {b.clear();return -7;}
    }
    b.pending=count!=0;
    return uh_processor_process(processor,nullptr,0,out,channels,frames);
    } catch (...) {p->parameters.clear();return UH_ERR_EXCEPTION;}
}
}
