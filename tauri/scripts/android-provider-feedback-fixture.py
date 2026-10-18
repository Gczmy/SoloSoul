from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import argparse,hashlib,json,threading

EXPECTED={'model':'fixture-model','messages':[{'role':'user','content':'Hello'}],'max_tokens':10}
KEY='Bearer public-unused'
PATHS={'/success/v1/chat/completions':200,'/error/v1/chat/completions':500}
def classify(path,authorization,body):
    if path not in PATHS or authorization!=KEY or type(body)is not dict or body!=EXPECTED:return 400
    return PATHS[path]

def create_server(port, destination):
    assert type(port)is int and 1024<=port<=65535
    requested=Path(destination).absolute()
    assert requested.parent.is_dir() and not requested.is_symlink() and not requested.exists()
    output=requested.resolve()
    rows=[];lock=threading.Lock()
    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            length=self.headers.get('Content-Length')
            if self.client_address[0]!='127.0.0.1' or length is None or not length.isdecimal() or not 0<int(length)<=2048:
                self.send_error(400);return
            raw=self.rfile.read(int(length))
            try:body=json.loads(raw)
            except (UnicodeError,ValueError):body=None
            status=classify(self.path,self.headers.get('Authorization'),body)
            with lock:
                rows.append(dict(number=len(rows)+1,path=self.path,status=status,onlyPublicGreeting=body==EXPECTED,
                    publicDummyKey=self.headers.get('Authorization')==KEY,bodySha256=hashlib.sha256(raw).hexdigest()))
                output.write_text(json.dumps(dict(boundHost='127.0.0.1',port=port,requests=rows),indent=2)+'\n',encoding='utf-8')
            data=json.dumps({'choices':[{'message':{'role':'assistant','content':'fixture response'}}]} if status==200 else {'error':'public fixture failure'}).encode()
            self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)))
            self.end_headers();self.wfile.write(data)
        def log_message(self,*args):pass
    return ThreadingHTTPServer(('127.0.0.1',port),Handler)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--port',type=int,required=True);parser.add_argument('--output',required=True)
    args=parser.parse_args();server=create_server(args.port,args.output)
    print('Owned loopback public provider response fixture ready',flush=True)
    server.serve_forever()
if __name__=='__main__':main()
