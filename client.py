import socket
import threading
import json
import sys

class Client(threading.Thread):
    def __init__(self, host, port, handle = None):
        self.host = host
        self.port = port
        if handle is None:
            self.handle = lambda b: print(b)
        else:
            self.handle = handle
        self.socket = socket.socket()
        self.socket.connect((self.host, self.port))
        self.working = True
        self.socket.settimeout(2)
        super(Client, self).__init__()

    def send(self, b: bytes):
        try:
            self.socket.send(b)
        except Exception as e:
            print(e)
            self.working = False

    def run(self):
        # output
        try:
            while self.working:
                try:
                    recv = self.socket.recv(8192)
                    if len(recv) == 0:
                        break
                    self.handle(recv)
                except socket.timeout:
                    continue
        except Exception as e:
            print(e)
        print(">>> Terminated.")


def main(args):
    host = '127.0.0.1'
    port = 15900
    if len(args) > 0:
        s = args[0]
        i = s.find(':')
        if i > 0:
            host = s[:i]
            port = int(s[i+1:])
        else:
            port = int(s)

    c = None
    try:
        c = Client(host, port)
        c.start()
        while c.working:
            s = input('> ')
            if len(s) == 0:
                continue
            try:
                if s.startswith('"'):
                    s = json.loads(s)
                b = s.encode('utf-8')
                c.send(b)
            except Exception as e:
                print(e)
    except KeyboardInterrupt:
        print("![KeyboardInterrupt]")
    
    print('>>> Terminating ...')
    c.working = False

if __name__ == "__main__":
    main(sys.argv[1:])