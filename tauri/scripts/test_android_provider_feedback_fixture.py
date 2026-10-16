from pathlib import Path
import copy,http.client,importlib.util,json,socket,tempfile,threading,unittest
S=Path(__file__).parent
spec=importlib.util.spec_from_file_location('fixture_server',S/'android-provider-feedback-fixture.py');server=importlib.util.module_from_spec(spec);spec.loader.exec_module(server)
class FixtureTests(unittest.TestCase):
    def test_real_loopback_responses_and_public_request_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'requests.json'
            with socket.socket() as reservation:
                reservation.bind(('127.0.0.1',0));port=reservation.getsockname()[1]
            listener=server.create_server(port,output)
            thread=threading.Thread(target=listener.serve_forever,daemon=True);thread.start()
            try:
                for path,key,status in [('/success/v1/chat/completions',server.KEY,200),('/error/v1/chat/completions',server.KEY,500),('/success/v1/chat/completions','Bearer rejected-unused',400)]:
                    connection=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
                    try:
                        connection.request('POST',path,json.dumps(server.EXPECTED),{'Authorization':key,'Content-Type':'application/json'})
                        response=connection.getresponse();self.assertEqual(response.status,status);body=json.loads(response.read())
                        if status==200:self.assertEqual(body['choices'][0]['message']['content'],'fixture response')
                        else:self.assertEqual(body,{'error':'public fixture failure'})
                    finally:connection.close()
                record=json.loads(output.read_text())
                self.assertEqual(record['boundHost'],'127.0.0.1')
                self.assertEqual([r['status'] for r in record['requests']],[200,500,400])
                self.assertEqual([r['number'] for r in record['requests']],[1,2,3])
                self.assertTrue(all(r['onlyPublicGreeting'] for r in record['requests']))
                self.assertEqual([r['publicDummyKey'] for r in record['requests']],[True,True,False])
            finally:
                listener.shutdown();listener.server_close();thread.join(timeout=5);self.assertFalse(thread.is_alive())
    def test_explicit_output_cannot_overwrite_an_existing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'existing.json';output.write_text('preserve')
            with self.assertRaises(AssertionError):server.create_server(27683,output)
            self.assertEqual(output.read_text(),'preserve')
    def test_exact_success_and_error_public_greeting(self):
        self.assertEqual(server.classify('/success/v1/chat/completions',server.KEY,server.EXPECTED),200)
        self.assertEqual(server.classify('/error/v1/chat/completions',server.KEY,server.EXPECTED),500)
    def test_any_other_path_key_or_body_is_rejected(self):
        variants=[('wrong',server.KEY,server.EXPECTED),('/success/v1/chat/completions','Bearer real-secret',server.EXPECTED),
                  ('/success/v1/chat/completions',server.KEY,None)]
        for key,value in [('model','other-model'),('messages',[{'role':'user','content':'private'}]),('max_tokens',100),('stream',True)]:
            body=copy.deepcopy(server.EXPECTED);body[key]=value;variants.append(('/success/v1/chat/completions',server.KEY,body))
        for path,key,body in variants:
            with self.subTest(path=path,body=body):self.assertEqual(server.classify(path,key,body),400)
if __name__=='__main__':unittest.main()
