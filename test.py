#!/usr/bin/env python3

# Steen Hegelund
# Time-Stamp: 2024-Oct-21 13:31
# vim: set ts=4 sw=4 sts=4 tw=120 cc=120 et ft=python :

import argparse
import re
import sys
import os
import signal
import time
from termswx import LinuxLoginMixin, LoggerMixin, MenuMixin, TerminalIo


def parse_arguments():
    parser = argparse.ArgumentParser()

    parser.add_argument('--verbose', '-v', action='count', default=0)
    parser.add_argument('--count', '-c', type=int, default=5)
    parser.add_argument('--duration', '-d', type=int, default=1, help='Duration in hours')
    parser.add_argument('--interval', '-i', type=int, default=600, help='Poll interval in seconds')
    parser.add_argument('username', type=str)
    parser.add_argument('password', type=str)

    return parser.parse_args()


class Commands(LinuxLoginMixin, LoggerMixin, MenuMixin, TerminalIo):
    date_rex = re.compile(r'\S+,\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+\+0000')
    meminfo_rex = re.compile(r'(MemFree):\s+(\S+)\s+(\S+)')
    if_rex = re.compile(r'(\S+):\s+(\S+):\s+\<(\S+)\>')

    def __init__(self):
        super().__init__()
        self.log('test.log')
        self.erase()
        self.stop = False

    def remaining(self, secs):
        hours = int(secs / 3600)
        secs = int(secs % 3600)
        mins = int(secs / 60)
        secs = int(secs % 60)
        return (hours, mins, secs)

    def set_date(self):
        cmd = f'date {time.strftime("%Y-%m-%d%H:%M:%S", time.localtime())}'
        res = self.command(cmd)
        if 'date: invalid date' in res[0]:
            cmd = f'date {time.strftime("%m%d%H%M%Y", time.localtime())}'
            res = self.command(cmd)
        self.add_log(cmd, res)

    def banner(self, idx, interval, timeout):
        hours, mins, secs = self.remaining(timeout - time.monotonic())
        text = f'{idx}: Sleep {interval}s, Remains: {hours}h {mins}m {secs}s'
        self.alert(text)
        self.add_log('Iteration', text)

    def get_date(self):
        res = self.command('date -uR')
        self.add_log('date', (res, self.parse_date(res)))

    def parse_date(self, lines):
        if len(lines) > 0:
            mt = self.date_rex.match(lines[0])
            if mt:
                return (mt[1], mt[2], mt[3], mt[4])
        return None

    def get_meminfo(self):
        res = self.command('cat /proc/meminfo')
        self.add_log('meminfo', self.parse_meminfo(res))

    def parse_meminfo(self, lines):
        for line in lines:
            mt = self.meminfo_rex.match(line)
            if mt:
                return (mt[1], mt[2], mt[3])
        return None

    def get_uname(self):
        self.cmd("uname -a")

    def get_interfaces(self):
        res = self.command('ip -c addr show')
        for line in res:
            mt = self.if_rex.match(line)
            if mt:
                iface = f'Interface {mt[2]} {mt[3]}'
                self.add_log('interface', iface)

    def get_list(self):
        self.cmd("ls -lah")

    def get_hostenv(self):
        self.add_log('python_version', sys.version)
        self.add_log('python_executable', sys.executable)
        env = []
        for elem in os.environ.items():
            env.append(str(elem))

        self.add_log('hostenv', env)

    def run(self, duration, interval):
        self.get_hostenv()
        idx = 1
        timeout = time.monotonic() + duration * 3600
        while time.monotonic() < timeout and not self.stop:
            self.get_list()
            self.get_date()
            self.get_meminfo()
            self.flush_log()
            self.banner(idx, interval, timeout)
            time.sleep(interval)
            idx += 1
        self.save()


def main():
    cmds = Commands()

    args = parse_arguments()

    cmds.login(args.username, args.password)
    cmds.set_date()

    def sig_handler(signum, frame):
        filename = cmds.save()
        raise SystemExit(f'\r\nStop after saving {filename}')

    signal.signal(signal.SIGTERM, sig_handler)

    menu = (
           ('Get uname', cmds.get_uname),
           ('Get Interfaces', cmds.get_interfaces),
           ('Run', cmds.run, args.duration, args.interval),
    )
    cmds.show_menu(menu, 'Choose > ', '----- Test Menu -----')


if __name__ == '__main__':
    main()
