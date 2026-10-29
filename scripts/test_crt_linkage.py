"""Reject runtime DLL imports in either binary of the static distribution."""
import contextlib
import io
import unittest
from unittest.mock import patch

import crt_linkage


class CrtLinkageTests(unittest.TestCase):
    def check(self, imports):
        with patch('sys.argv', ['crt_linkage.py', 'app.exe', 'uapmd_shim.dll']), \
                patch.object(crt_linkage, 'dumpbin', return_value='dumpbin'), \
                patch.object(crt_linkage, 'dependents', side_effect=imports), \
                contextlib.redirect_stdout(io.StringIO()):
            return crt_linkage.main()

    def test_static_binaries_may_import_windows_system_dlls(self):
        self.assertEqual(self.check([['KERNEL32.dll'], ['USER32.dll', 'ole32.dll']]), 0)

    def test_runtime_import_in_either_binary_fails(self):
        for dll in ['VCRUNTIME140.dll', 'MSVCP140.dll', 'CONCRT140.dll', 'vccorlib140.dll',
                    'ucrtbase.dll', 'api-ms-win-crt-runtime-l1-1-0.dll',
                    'VCRUNTIME140D.dll', 'MSVCP140D.dll', 'ucrtbased.dll']:
            for binary in range(2):
                with self.subTest(dll=dll, binary=binary):
                    imports = [[], []]
                    imports[binary] = [dll]
                    self.assertEqual(self.check(imports), 1)


if __name__ == '__main__':
    unittest.main()
