#!/usr/bin/env python3

# Steen Hegelund
# Time-Stamp: 2024-Sep-15 17:11
# vim: set ts=4 sw=4 sts=4 tw=120 cc=120 et ft=python :

import argparse
import re
import sys
import os
import signal
import time
import termswx


def parse_arguments():
    parser = argparse.ArgumentParser()

    parser.add_argument('--verbose', '-v', action='count', default=0)
    parser.add_argument('--count', '-c', type=int, default=5)
    parser.add_argument('username', type=str)
    parser.add_argument('password', type=str)

    return parser.parse_args()


class Commands(termswx.LinuxLoginMixin, termswx.LoggerMixin, termswx.MenuMixin, termswx.TerminalIo):
    date_rex = re.compile(r'\S+,\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+\+0000')

    def __init__(self):
        super().__init__()
        self.log('test.log')

    def banner(self, idx, total):
        text = f'This is iteration {idx} of {total}'
        self.alert(text)
        self.log_responses.append(text)

    def get_date(self):
        res = self.command('date -uR')
        self.alert(f'date: {res}')
        self.log_responses.append((res, self.parse_date(res)))

    def parse_date(self, lines):
        if len(lines) > 0:
            mt = self.date_rex.match(lines[0])
            if mt:
                return (mt[1], mt[2], mt[3], mt[4])
        return None

    def get_meminfo(self):
        self.cmd('cat /proc/meminfo')

    def get_uname(self):
        self.cmd("uname -a")

    def get_interfaces(self):
        res = self.command('ip -c addr show')
        if_rex = re.compile(r'(\S+):\s+(\S+):\s+\<(\S+)\>')
        for line in res:
            mt = if_rex.match(line)
            if mt:
                iface = f'Interface {mt[2]} {mt[3]}'
                self.log_responses.append(iface)

    def get_list(self):
        self.cmd("ls -lah")

    def set_date(self):
        cmd = f'date {time.strftime("%Y-%m-%d%H:%M:%S", time.localtime())}'
        res = self.command(cmd)
        if 'date: invalid date' in res[0]:
            cmd = f'date {time.strftime("%m%d%H%M%Y", time.localtime())}'
            res = self.command(cmd)
        self.log_responses.append((cmd, res))

    def choice(self):
        self.alert('Do you want to continue?> ')
        res = self.read_line(60.0)
        self.alert(f'Got response {res}')
        if 'yes' not in res.lower():
            sys.exit(0)

    def run(self, count):
        cnt = 1
        while cnt <= count:
            self.banner(cnt, count)
            self.get_list()
            self.get_date()
            self.get_meminfo()
            cnt += 1

        for elem in os.environ.items():
            self.log_responses.append(str(elem))

        self.save()


def main():
    cmds = Commands()

    args = parse_arguments()

    cmds.login(args.username, args.password)

    def sig_handler(signum, frame):
        filename = cmds.save()
        raise SystemExit(f'\r\nStop after saving {filename}')

    signal.signal(signal.SIGTERM, sig_handler)

    menu = (
           ('Get uname', cmds.get_uname),
           ('Get Interfaces', cmds.get_interfaces),
           ('Run', cmds.run, args.count),
    )
    cmds.show_menu(menu, 'Choose > ', '----- Test Menu -----')


if __name__ == '__main__':
    main()
