#include "public.sdk/source/vst/vstsinglecomponenteffect.h"
#include "public.sdk/source/main/pluginfactory.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstmidicontrollers.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/base/ibstream.h"
using namespace Steinberg;
using namespace Steinberg::Vst;
static const FUID fixtureId(0x13213213, 0x12341234, 0x56785678, 0xABCDEF01);
class Fixture : public SingleComponentEffect, public IMidiMapping {
    double values[2]{0.2, 0.3};
public:
    uint32 PLUGIN_API addRef() override { return SingleComponentEffect::addRef(); }
    uint32 PLUGIN_API release() override { return SingleComponentEffect::release(); }
    tresult PLUGIN_API queryInterface(const TUID iid, void** obj) override {
        QUERY_INTERFACE(iid,obj,IMidiMapping::iid,IMidiMapping)
        return SingleComponentEffect::queryInterface(iid,obj);
    }
    tresult PLUGIN_API getMidiControllerAssignment(int32 bus,int16 channel,CtrlNumber cc,ParamID& id) override {
        if (bus!=0 || channel<0 || channel>15) return kResultFalse;
        if (cc==7) {id=101;return kResultOk;}
        if (cc==ControllerNumbers::kAfterTouch) {id=135;return kResultOk;}
        if (cc==ControllerNumbers::kPitchBend && channel!=15) {id=118;return kResultOk;}
        return kResultFalse;
    }
    static FUnknown* create(void*) { return static_cast<IComponent*>(new Fixture); }
    tresult PLUGIN_API initialize(FUnknown* context) override {
        auto result = SingleComponentEffect::initialize(context);
        if (result != kResultOk) return result;
        addAudioOutput(STR16("Output"), SpeakerArr::kStereo); addEventInput(STR16("Notes"));
        parameters.addParameter(STR16("Same name"), STR16("native"), 0, 0.2, ParameterInfo::kCanAutomate, 101);
        parameters.addParameter(STR16("Same name"), STR16("native"), 0, 0.3, ParameterInfo::kCanAutomate, 118);
        parameters.addParameter(STR16("Read only"), nullptr, 0, 0, ParameterInfo::kIsReadOnly, 135);
        parameters.addParameter(STR16("Stepped"), nullptr, 3, 0, ParameterInfo::kCanAutomate, 152);
        return kResultOk;
    }
    tresult PLUGIN_API setProcessing(TBool) override { return kResultOk; }
    tresult PLUGIN_API getState(IBStream* stream) override {
        int32 written{};return stream && stream->write(values,sizeof(values),&written)==kResultOk && written==sizeof(values) ? kResultOk : kResultFalse;
    }
    tresult PLUGIN_API setState(IBStream* stream) override {
        int32 read{};double state[2];
        if (!stream || stream->read(state,sizeof(state),&read)!=kResultOk || read!=sizeof(state)) return kResultFalse;
        for (int i=0;i<2;++i) {values[i]=state[i];setParamNormalized(i==0?101:118,state[i]);}
        return kResultOk;
    }
    tresult PLUGIN_API setComponentState(IBStream* stream) override {return setState(stream);}
    // Nonlinear/native range differs from normalized storage. Audio automation
    // must never call the controller's plain->normalized conversion.
    ParamValue PLUGIN_API normalizedParamToPlain(ParamID id, ParamValue value) override {
        return id == 101 || id == 118 ? 100.0 + 900.0 * value * value : value;
    }
    ParamValue PLUGIN_API plainParamToNormalized(ParamID id, ParamValue value) override {
        return id == 101 || id == 118 ? (value - 100.0) / 900.0 : value;
    }
    tresult PLUGIN_API process(ProcessData& block) override {
        bool invalid = false;
        if (auto* changes = block.inputParameterChanges)
            for (int32 i = 0; i < changes->getParameterCount(); ++i) {
                auto* q = changes->getParameterData(i);
                auto id = q->getParameterId();
                for (int32 j = 0; j < q->getPointCount(); ++j) {
                    int32 offset; double value; q->getPoint(j, offset, value);
                    invalid |= offset != 0;
                    if (id == 101) values[0] = value;
                    else if (id == 118) values[1] = value;
                    else invalid = true;
                }
            }
        if (block.inputEvents)
            for (int32 i=0;i<block.inputEvents->getEventCount();++i) {
                Event e{};block.inputEvents->getEvent(i,e);
                if (e.type==Event::kNoteOnEvent && e.noteOn.pitch==60) invalid |= values[0] != 0.7 || values[1] != 0.8;
            }
        if (auto* output = block.outputParameterChanges)
            for (int32 i = 0; i < 2; ++i) {
                int32 index;
                auto* queue = output->addParameterData(1000 + i, index);
                if (!queue || queue->addPoint(0, values[i], index) != kResultOk) invalid = true;
            }
        for (int32 c = 0; c < 2; ++c)
            for (int32 i = 0; i < block.numSamples; ++i)
                block.outputs[0].channelBuffers32[c][i] = invalid ? -1.f : float(values[c]);
        return kResultOk;
    }
};
bool InitModule() { return true; }
bool DeinitModule() { return true; }
BEGIN_FACTORY_DEF("cat test", "", "")
DEF_CLASS2(INLINE_UID_FROM_FUID(fixtureId), PClassInfo::kManyInstances, kVstAudioEffectClass,
    "Parameter fixture", Vst::kDistributable, "Instrument", "1", kVstVersionString, Fixture::create)
END_FACTORY
