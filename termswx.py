#!/usr/bin/env python3

# Steen Hegelund
# Read and write from stdin/stdout and send commands and receive responses
# Time-Stamp: 2024-Sep-11 21:47
# vim: set ts=4 sw=4 sts=4 tw=120 cc=120 et ft=python :

import re
import sys
import time
import os
import json
import queue
import threading

# Stderr Byte Prefixes
ALERT = 0x11
MENU_TITLE = 0x12
MENU_ITEM = 0x13
MENU_PROMPT = 0x14
MENU_SELECTED = 0x15


class TerminalIo:
    def __init__(self, waitfor=r'[#$] ', add_cr=False, poll_timeout=0.5):
        self.waitfor = waitfor
        self.add_cr = add_cr
        self.poll_timeout = poll_timeout
        self.lines = []
        self.queue = queue.Queue()
        threading.Thread(target=self.reader, daemon=True).start()

    def reader(self):
        while True:
            try:
                ch = os.read(sys.stdin.fileno(), 1).decode()
            except UnicodeDecodeError:
                ch = ' '
            self.queue.put(ch)

    def poll_stdin(self):
        try:
            return self.queue.get(block=True, timeout=self.poll_timeout)
        except queue.Empty:
            return None

    def flush_stdin(self, timeout):
        timeout += time.monotonic()
        while time.monotonic() < timeout:
            self.poll_stdin()

    def read_response(self, waitfor=None, timeout=None, keep=False):
        self.lines = []
        token = re.compile(self.waitfor)
        if waitfor is not None:
            token = re.compile(waitfor)

        if timeout:
            timeout += time.monotonic()

        line = ''
        echo = True
        while True:
            ch = self.poll_stdin()
            if ch is not None:
                if ch == '\r':
                    if echo:
                        line = ''
                        echo = False
                elif ch == '\n':
                    if len(line):
                        self.lines.append(line)
                        line = ''
                elif ch == '\b':
                    line = line[:-1]
                else:
                    line += ch
                    if token.search(line):
                        if keep:
                            self.lines.append(line)
                        break
            if timeout and time.monotonic() > timeout:
                return None

        return self.lines

    def read_line(self, timeout=None):
        self.lines = []
        if timeout:
            timeout += time.monotonic()

        line = ''
        while True:
            ch = self.poll_stdin()
            if ch is not None:
                if ch == '\r' or ch == '\n':
                    if len(line):
                        return line
                    return None
                elif ch == '\b':
                    line = line[:-1]
                else:
                    line += ch
            if timeout and time.monotonic() > timeout:
                return None

    def write_command(self, line):
        if self.add_cr:
            print(line + '\r')
        else:
            print(line)

    def command(self, line, waitfor=None, timeout=None, keep=False):
        self.write_command(line)
        return self.read_response(waitfor, timeout, keep)

    def alert(self, line):
        print(f'\x11{line}', file=sys.stderr)


class LoggerMixin:
    def log(self, name):
        self.log_filename = os.path.abspath(name)
        self.log_responses = []

    def save(self):
        with open(self.log_filename, 'at') as fobj:
            fobj.write(json.dumps(self.log_responses, indent=4, sort_keys=True))
        self.alert(f'Saving {self.log_filename}')
        return self.log_filename

    def erase(self):
        try:
            os.remove(self.log_filename)
        except FileNotFoundError:
            pass

    def add_log(self, req, res):
        self.log_responses.append((req, res))
        return res

    def cmd(self, line):
        res = self.command(line)
        self.log_responses.append((line, res))
        return res


class LinuxLoginMixin:
    def login(self, user, passwd=''):
        count = 3
        while count > 0:
            res = self.command('', r'(login: |[#$] )', 1.0, True)
            if res:
                if 'login: ' in res[0] or (len(res) > 1 and 'login: ' in res[1]):
                    res = self.command(user, r'(Password: |[#$] )', 1.0, True)
                    if res is not None:
                        self.command(passwd)
                        return True
                elif 'Password: ' in res[0]:
                    pass
                elif '# ' in res[0]:
                    return False
            count -= 1

        return False


class MenuMixin:
    def menu_title(self, text):
        print(f'\x12{text}', file=sys.stderr)

    def menu_item(self, text):
        print(f'\x13{text}', file=sys.stderr)

    def prompt(self, text):
        print(f'\x14{text}', file=sys.stderr)

    def selected(self, text):
        print(f'\x15{text}', file=sys.stderr)

    def show_menu(self, menu, prompt=None, title='=== Menu ==='):
        self.menu_title(title)
        for (idx, (text, method, *args)) in enumerate(menu):
            self.menu_item(f'{idx + 1}: {text}')
        if prompt is not None:
            self.prompt(prompt)
        else:
            self.prompt('Select > ')
        self.flush_stdin(0.5)
        res = self.read_line(60.0)
        for (idx, (text, method, *args)) in enumerate(menu):
            if res == str(idx + 1):
                self.selected(f'Starting: {text}')
                method(*args)
                return
