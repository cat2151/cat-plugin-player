"""Device-free ABI contract, against isolated bundles passed explicitly on CLI."""
import ctypes as c
from pathlib import Path
import sys
import time


class Info(c.Structure):
    _fields_ = [('generation', c.c_uint64), ('revision', c.c_uint64),
                ('index', c.c_uint32), ('eligible', c.c_uint32), ('normalized', c.c_uint32),
                ('minimum', c.c_double), ('maximum', c.c_double), ('current', c.c_double),
                ('stable_id', c.c_char * 256), ('name', c.c_char * 1024), ('path', c.c_char * 2048)]


class Value(c.Structure):
    _fields_ = [('generation', c.c_uint64), ('revision', c.c_uint64),
                ('index', c.c_uint32), ('value', c.c_double)]

class Event(c.Structure):
    _fields_ = [('parameter', Value), ('midi', c.c_uint32), ('kind', c.c_uint32)]


def test(dll_path, clap_path, vst3_path):
    dll = c.CDLL(str(Path(dll_path).resolve()))
    P, I, U, S = c.c_void_p, c.c_int32, c.c_uint32, c.c_char_p
    WAKE, DONE = c.CFUNCTYPE(None, P), c.CFUNCTYPE(None, P, I, S)
    signatures = [
        ('uh_create', [WAKE, P], P), ('uh_destroy', [P], None),
        ('uh_pump', [P], None), ('uh_restore_plugin', [P, S, S, S, S, S], I),
        ('uh_instance_create', [P, I, U, U, DONE, P], None), ('uh_instance_destroy', [P, I], None),
        ('uh_parameter_count', [P, I], I), ('uh_parameter_info', [P, I, U, c.POINTER(Info)], I),
        ('uh_parameter_text', [P, I, c.POINTER(Value), P, I], I),
        ('uh_processor_create', [P, I, U, U], P), ('uh_processor_destroy', [P], None),
        ('uh_processor_prepare_parameters', [P], I),
        ('uh_processor_parameter_batch', [P, c.POINTER(Value), U], I),
        ('uh_processor_process', [P, P, I, P, I, I], I),
        ('uh_processor_process_automation', [P, c.POINTER(Event), U, P, I, I], I),
    ]
    for name, args, result in signatures:
        fn = getattr(dll, name)
        fn.argtypes, fn.restype = args, result
    wake = WAKE(lambda _: None)
    host = dll.uh_create(wake, None)
    assert host
    try:
        for format_name, path, plugin_id in [('CLAP', clap_path, 'cat.test.parameters'),
                                          ('VST3', vst3_path, '132132131234123456785678ABCDEF01')]:
            plugin = dll.uh_restore_plugin(host, format_name.encode(), plugin_id.encode(),
                                          b'Parameter fixture', b'cat test', str(Path(path).resolve()).encode())
            assert plugin >= 0
            instances = []
            processors = []
            try:
                for _ in range(2):
                    done = []
                    callback = DONE(lambda _, instance, error: done.append((instance, error)))
                    dll.uh_instance_create(host, plugin, 48000, 64, callback, None)
                    deadline = time.monotonic() + 20
                    while not done:
                        assert time.monotonic() < deadline, 'creation timeout'
                        dll.uh_pump(host)
                        time.sleep(.001)
                    instance, error = done[0]
                    assert instance >= 0 and not error, error
                    instances.append(instance)
                    processor = dll.uh_processor_create(host, instance, 48000, 64)
                    assert processor
                    processors.append(processor)
                    assert dll.uh_processor_prepare_parameters(processor) == 0
                count = dll.uh_parameter_count(host, instances[0])
                metadata = []
                for index in range(count):
                    info = Info()
                    assert dll.uh_parameter_info(host, instances[0], index, c.byref(info)) == 0
                    metadata.append(info)
                assert [p.eligible for p in metadata] == ([1, 1, 0, 0, 0] if format_name == 'CLAP' else [1, 1, 0, 0])
                assert [p.stable_id for p in metadata[:2]] == [b'101', b'118']
                first = metadata[0]
                assert first.normalized == (format_name == 'VST3')
                assert abs(first.current - .2) < 1e-6
                values = (Value * 2)(Value(first.generation, first.revision, 0, .7),
                                     Value(first.generation, first.revision, 1, .8))
                # Validate all before enqueue: rejected batches cannot leak a partial first update.
                for bad in [float('nan'), float('inf'), -1., 2.]:
                    values[1].value = bad
                    assert dll.uh_processor_parameter_batch(processors[0], values, 2) < 0
                values[1].value = .8
                assert dll.uh_processor_parameter_batch(processors[1], values, 2) == -6
                assert dll.uh_processor_parameter_batch(processors[0], values, 257) == -7
                values[1].index = 0
                assert dll.uh_processor_parameter_batch(processors[0], values, 2) == -3
                values[1].index = 2
                assert dll.uh_processor_parameter_batch(processors[0], values, 2) == -3
                values[1].index = 1
                text = c.create_string_buffer(2048)
                assert dll.uh_parameter_text(host, instances[0], c.byref(values[0]), text, len(text)) == 0
                assert text.value
                assert dll.uh_processor_parameter_batch(processors[0], values, 2) == 0
                notes = (U * 2)(0x40903C00, 0xFFFF0000)
                output = (c.c_float * 128)()
                assert dll.uh_processor_process(processors[0], notes, 2, output, 2, 64) == 0
                assert abs(output[0] - .7) < 1e-6 and abs(output[1] - .8) < 1e-6, list(output[:2])
                applied = Info()
                assert dll.uh_parameter_info(host, instances[0], 0, c.byref(applied)) == 0
                assert abs(applied.current - .7) < 1e-6, ('applied current value', applied.current)
                ordered = (Event * 6)(
                    Event(Value(first.generation, first.revision, 0, .2), 0, 0),
                    Event(Value(first.generation, first.revision, 1, .3), 0, 0),
                    Event(Value(), 0x20903D64, 1),
                    Event(Value(first.generation, first.revision, 0, .7), 0, 0),
                    Event(Value(first.generation, first.revision, 1, .8), 0, 0),
                    Event(Value(), 0x20903C64, 1))
                ordered[4].parameter.index = 2
                assert dll.uh_processor_process_automation(processors[0], ordered, 6, output, 2, 64) == -3
                ordered[4].parameter.index = 1
                assert dll.uh_processor_process_automation(processors[0], ordered, 513, output, 2, 64) == -7
                assert dll.uh_processor_process_automation(processors[0], ordered, 6, output, 2, 64) == 0
                assert abs(output[0] - .7) < 1e-6 and abs(output[1] - .8) < 1e-6
                if format_name == 'VST3':
                    overflow = (Event * 65)(*[Event(Value(first.generation, first.revision, 0, .4), 0, 0) for _ in range(65)])
                    assert dll.uh_processor_process_automation(processors[0], overflow, 65, output, 2, 64) == -7
                    unchanged = Info()
                    assert dll.uh_parameter_info(host, instances[0], 0, c.byref(unchanged)) == 0
                    assert abs(unchanged.current - .7) < 1e-6, 'rejected values were published as current'
                assert dll.uh_processor_process(processors[1], None, 0, output, 2, 64) == 0
                assert abs(output[0] - .2) < 1e-6 and abs(output[1] - .3) < 1e-6
                if format_name == 'CLAP':
                    values[0].value = .91
                    assert dll.uh_processor_parameter_batch(processors[0], values, 2) == 0
                    assert dll.uh_processor_process(processors[0], None, 0, output, 2, 64) == 0
                    for _ in range(10):
                        dll.uh_pump(host)
                    refreshed = Info()
                    assert dll.uh_parameter_info(host, instances[0], 0, c.byref(refreshed)) == 0
                    assert refreshed.revision > first.revision, 'rescan not observed'
                    assert dll.uh_processor_parameter_batch(processors[0], values, 2) == -6
                    assert dll.uh_parameter_text(host, instances[0], c.byref(values[0]), text, len(text)) == -6
                    assert dll.uh_processor_prepare_parameters(processors[0]) == 0
                    for v in values:
                        v.revision = refreshed.revision
                    assert dll.uh_processor_parameter_batch(processors[0], values, 2) == 0
                    assert dll.uh_processor_process(processors[0], None, 0, output, 2, 64) == 0
                print(format_name, 'metadata/display/batch/instance identity/capacity/rejected values/order PASS', flush=True)
            finally:
                for processor in reversed(processors):
                    dll.uh_processor_destroy(processor)
                for instance in reversed(instances):
                    dll.uh_instance_destroy(host, instance)
    finally:
        dll.uh_destroy(host)


if __name__ == '__main__':
    test(*sys.argv[1:])
