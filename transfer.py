#!/usr/bin/env python3.11

# Steen Hegelund
# Time-Stamp: 2024-Sep-24 20:03
# vim: set ts=4 sw=4 sts=4 tw=120 cc=120 et ft=python :

import sys
import os
import xmodem
import time
import queue
import threading
import argparse

# Stderr Byte Prefixes
ALERT = 0x11
MENU_TITLE = 0x12
MENU_ITEM = 0x13
MENU_PROMPT = 0x14
MENU_SELECTED = 0x15


def parse_arguments():
    parser = argparse.ArgumentParser()

    parser.add_argument('--send', '-s', action='store_true')
    parser.add_argument('remotefile', type=str)
    parser.add_argument('localfile', type=str)

    return parser.parse_args()


class XModem:
    def __init__(self):
        self.poll_timeout = 0.5
        self.lines = []
        self.queue = queue.Queue()
        threading.Thread(target=self.reader, daemon=True).start()
        self.modem = xmodem.XMODEM(self.getc, self.putc)
        self.out = os.fdopen(sys.stdout.fileno(), 'wb')

    def reader(self):
        while True:
            try:
                ch = os.read(sys.stdin.fileno(), 1)
            except UnicodeDecodeError:
                ch = b' '
            self.queue.put(ch)

    def getc(self, size, timeout=1):
        idx = 0
        data = b''
        timeout += time.monotonic()
        while idx < size:
            try:
                data += self.queue.get(block=True, timeout=self.poll_timeout)
                idx += 1
            except queue.Empty:
                pass
            if time.monotonic() >= timeout:
                return None
        return data

    def putc(self, ch, timeout=1):
        self.out.write(ch)
        self.out.flush()
        return 1

    def read_line(self):
        line = b''
        while True:
            ch = self.getc(1)
            if ch is not None:
                line += ch
                if ch == b'\n' or ch == b'\r':
                    break
        return line

    def recv_file(self, remote, local):
        print(f'lsz -Xq -C 4 {remote}', file=sys.stdout)
        for idx in range(0, 2):
            print(f'\x11{self.read_line()}', file=sys.stderr)
        print('\x16', file=sys.stderr)
        time.sleep(1)
        with open(local, 'wb') as stream:
            self.modem.recv(stream)
        print('\x17', file=sys.stderr)
        print(f'\x11Transferred {remote} to {local}', file=sys.stderr)

    def send_file(self, remote, local):
        print(f'lrz -Xq {remote}', file=sys.stdout)
        for idx in range(0, 3):
            print(f'\x11{self.read_line()}', file=sys.stderr)
        print('\x16', file=sys.stderr)
        time.sleep(1)
        with open(local, 'rb') as stream:
            self.modem.send(stream)
        print('\x17', file=sys.stderr)
        print(f'\x11Transferred {local} to {remote}', file=sys.stderr)


def main():
    print(f'\x11Python: {sys.executable}', file=sys.stderr)
    args = parse_arguments()

    cmds = XModem()
    if args.send:
        cmds.send_file(args.remotefile, args.localfile)
    else:
        cmds.recv_file(args.remotefile, args.localfile)


if __name__ == '__main__':
    main()
