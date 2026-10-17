from types import SimpleNamespace
import unittest

import rf312_startup_ui as ui

PATH='/sdcard/rf312-ipc-'+'a'*32+'.xml'
XML=b'<?xml version="1.0"?><hierarchy rotation="0"><node package="com.solosoul.app" text="No account"/></hierarchy>'


def response(stdout=b'',stderr=b'',code=0):return SimpleNamespace(returncode=code,stdout=stdout,stderr=stderr)


class StartupUIReadinessTests(unittest.TestCase):
    def setUp(self):self.success=response(('UI hierchary dumped to: '+PATH+'\r\n').encode())

    def test_bound_success_reads_and_parses_exact_native_hierarchy(self):
        calls=[]
        def call(label,args,**kwargs):
            calls.append((label,args));return self.success if len(calls)==1 else response(XML)
        tree,raw=ui.read_tree(call,PATH)
        self.assertEqual(tree.tag,'hierarchy');self.assertEqual(raw,XML)
        self.assertEqual(calls[1][1],['exec-out','cat',PATH])

    def test_zero_exit_null_root_wait_never_reads_missing_xml(self):
        calls=[]
        def call(label,args,**kwargs):
            calls.append(label);return response(stderr=(ui.NULL_ROOT+'\r\n').encode())
        self.assertIsNone(ui.read_tree(call,PATH));self.assertEqual(calls,['startup-ui'])

    def test_unknown_errors_and_conflicting_success_are_not_retryable(self):
        for value in [response(stderr=b'permission denied'),response(stderr=ui.NULL_ROOT.encode(),code=1),
                      response(self.success.stdout,ui.NULL_ROOT.encode()),response(),response(code=2)]:
            with self.subTest(value=value),self.assertRaises(ValueError):ui.dump_ready(value,PATH)

    def test_success_for_another_nonce_or_non_owned_path_is_rejected(self):
        with self.assertRaises(ValueError):ui.dump_ready(self.success,PATH.replace('a'*32,'b'*32))
        with self.assertRaises(ValueError):ui.dump_ready(self.success,'/sdcard/account.xml')

    def test_missing_file_error_with_zero_exec_exit_is_not_xml(self):
        count=0
        def call(*args,**kwargs):
            nonlocal count;count+=1
            return self.success if count==1 else response(('cat: '+PATH+': No such file or directory\n').encode())
        with self.assertRaisesRegex(ValueError,'not complete XML'):ui.read_tree(call,PATH)

    def test_invalid_xml_wrong_root_and_cat_stderr_remain_failures(self):
        for value in [response(b'<hierarchy>'),response(b'<other/>'),response(XML,b'error')]:
            rows=iter([self.success,value])
            with self.subTest(value=value),self.assertRaises(ValueError):ui.read_tree(lambda *a,**k:next(rows),PATH)

    def test_a_later_valid_dump_is_allowed_after_exact_null_root(self):
        rows=iter([response(stderr=ui.NULL_ROOT.encode()),self.success,response(XML)])
        call=lambda *a,**k:next(rows)
        self.assertIsNone(ui.read_tree(call,PATH));self.assertEqual(ui.read_tree(call,PATH)[1],XML)

    def test_boolean_exit_or_non_byte_streams_do_not_coerce(self):
        for value in [response(self.success.stdout,code=False),response('text'),response(stderr='text')]:
            with self.subTest(value=value),self.assertRaises(ValueError):ui.dump_ready(value,PATH)


if __name__=='__main__':unittest.main()
