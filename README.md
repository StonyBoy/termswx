# termswx

The Terminal Switch helps you connect to serial port devices like e.g. small
embedded systems running Linux and allows you to expose the serial connection to
a local client and a number of remote clients via a TCP server.

The Terminal Switch also allows you to automate your tasks using a plain command
script (e.g. bash script) or a Python3 script running on your developer machine.

A Python3 script can also use the accompanying termswx.py helper script to
facilitate system login, waiting for responses, logging communication and
display a menu and run menu sub commands.

The Terminal Switch can be used on the 3 major platforms: Linux, macOS and
Windows as this was one of the goals for the project to have a multiplatform
terminal program with advanced scripting facilities in a popular scripting
language.

![Example Session](/documentation/termswx_script_menu.png "Running a Python3 script")

# Installation

You will need a rust development system to build and install `termswx` and
python3 if you want to run python scripts.

Follow the instructions at <https://www.rust-lang.org/tools/install> to install
rustup and then use that to install the development system.

## Supported platforms

The Terminal Switch has been tested on the following platforms:

| OS | Version | Rust Version | Python Version |
| :---- | :---- | :---- | :---- |
| Linux | Arch Linux | 1.81.0 | 3.11 |
| Linux | Ubuntu 22.04 | 1.81.0 | 3.11 |
| macOS | Sonoma 14.6.1 | 1.81.0 | 3.11 |
| macOS | Sequoia 15.0 | 1.81.0 | 3.11 |
| Windows | 10 Home 22H2 | 1.81.0 | 3.12 |
| Windows | 11 Pro 23H2 | 1.81.0 | 3.12 |

The serial connection was done with a "Future Technology Devices International,
Ltd FT232 Serial (UART) IC" USB/UART cable connected to a BeagleBone Black and a
Raspberry Pi 4 Compute Module as well as other Linux Embedded devices.

## Building the executable

You build the `termswx`executable with this command:

    cargo build

This will install all the dependencies and build these and finally build the
executable.

You can now run the `termswx`executable to show its help text with this command:

    cargo run -- --help

You can also install the program in your own user profile with the command:

    cargo install --path .

This will build a release (no debug) version of `termswx`and install it in
`~/.cargo/bin` (or the equivalent for your platform).

To be able to run the program directly you will need to have this folder in your
path.  This should already have been set up by the `rustup` installer.

# Examples

Here are some example on how to use the `termswx` program.  Here it is assumed
that you have installed `termswx` in your path.

## Making connections

* Show available serial ports

        termswx -e

* Connect to serial port /dev/ttyUSB0

        termswx /dev/ttyUSB0

* Connect to serial port /dev/ttyUSB1 and allow clients on TCP port 7273

        termswx /dev/ttyUSB1 -p 7273

* Connect to a termswx server name orion remotely on port 7273

        termswx orion:7273

* Exiting the program

    The default keybinding to exit the program is:

        Ctrl-q

## Automation

You have 3 possible ways to automate tasks with `termswx`:

1. Bind a key combination to send a string of text (Injection)
1. Bind a key combination to send the contents of a file (FileInjection)
1. Bind a key combination to send execute a Python3 script sending its stdout
   (RunScript)

### Sending text

You just use the appropriate key combination send the pre-configured text.  The
`ctrl+o` combination is configured to send the text `cat /proc/meminfo` which
will show a dump of the current memory usage on a Linux system.

You can of course change this to suit your own needs, as this is just
pre-configured as an example.

### Running a command file

Again you just hit the appropriate key combination and the lines in found in a
existing file will be sent one by one to the target device.  The pre-configured
example is the `ctrl+f` key combination that uses the `test.sh` file to send
these lines:

    date uname -a cd /usr ls -lah echo "TERM is $TERM" echo "TERM_SIZE is
    $TERM_SIZE" echo "USER is $USER" cd ~ echo "Content of ~/.profile is" cat
    .profile

If you send them to a Linux system you will get system infomation, the content
of the `/usr` folder, the values of some environment variables and content of
the `~/.profile` file on the target system.

> [!WARNING] If your current folder is not the folder where the `test.sh` file
> is located you will get an error, as the `termswx` configuration file does not
> know where the program was installed.  So for now make sure that your current
> folder is where the `test.sh` file is located.  You can change this later to
> include the absolute path of the command file.

### Running a Python3 script

The `ctrl+p` key combination runs the `test.py` python3 script using a python3
interpreter on your machine (so you need to have one installed to use this
feature).

This test script will try to login (if needed) and then show a small menu and
wait for you to select an entry:


If you select 2 (on a Linux Target system) you will get a list of the network
interfaces on the target system.

You type 2 and hit enter to continue and after the sub command has been executed
the script will end.

### Stopping scripts

If a script gets stuck waiting for response, you can cancel the execution using
the `ctrl+x` key combination.

# Configuration

The first time you run `termswx` the program will create a default configuration
file in a new termswx folder under the configuration folder for your system.  On
Linux and macOS this will be `~/.config/termswx/` and on Windows it will be
`%USERPROFILE%\AppData\Roaming\termswx\`.

This is the content of the default configuration file:

    [environment]
    TERM = "xterm"

    [general]
    version = 5

    [keymap]
    "Ctrl+b" = "break"
    "Ctrl+e" = "environment"
    "Ctrl+f" = "file test.sh"
    "Ctrl+o" = """
    inject cat /proc/meminfo
    """
    "Ctrl+p" = "run test.py --count 2 username password"
    "Ctrl+q" = "quit"
    "Ctrl+t" = "sttysize"
    "Ctrl+w" = "help"
    "Ctrl+x" = "stop"
    Del = "inject \b"
    Pause = "break"
    Print = "nop"
    Scroll = "nop"

    [keynames]
    "Ctrl+a" = "\u0001"
    "Ctrl+b" = "\u0002"
    "Ctrl+c" = "\u0003"
    "Ctrl+d" = "\u0004"
    "Ctrl+e" = "\u0005"
    "Ctrl+f" = "\u0006"
    "Ctrl+g" = "\u0007"
    "Ctrl+h" = "\b"
    "Ctrl+i" = "\t"
    "Ctrl+j" = """

    """
    "Ctrl+k" = "\u000B"
    "Ctrl+l" = "\f"
    "Ctrl+m" = "\r"
    "Ctrl+n" = "\u000E"
    "Ctrl+o" = "\u000F"
    "Ctrl+p" = "\u0010"
    "Ctrl+q" = "\u0011"
    "Ctrl+r" = "\u0012"
    "Ctrl+s" = "\u0013"
    "Ctrl+t" = "\u0014"
    "Ctrl+u" = "\u0015"
    "Ctrl+v" = "\u0016"
    "Ctrl+w" = "\u0017"
    "Ctrl+x" = "\u0018"
    "Ctrl+y" = "\u0019"
    "Ctrl+z" = "\u001A"
    Del = "\u007F"
    F1 = "\u001BOP"
    F10 = "\u001B[21~"
    F11 = "\u001B[23~"
    F12 = "\u001B[24~"
    F2 = "\u001BOQ"
    F3 = "\u001BOR"
    F4 = "\u001BOS"
    F5 = "\u001B[15~"
    F6 = "\u001B[17~"
    F7 = "\u001B[18~"
    F8 = "\u001B[19~"
    F9 = "\u001B[20~"
    Pause = "\u001B[57362u"
    Print = "\u001B[57361u"
    Scroll = "\u001B[57359u"

The most interesting section is the `[keymap]` section where you can configure
which key combinations activate which commands.

The `termswx` program has a number of built-in command that you can use:

| Command | Description |
|:----|:----|
| _quit_ | Exit the termswx program  |
| _inject_ <string> | Inject a single command line |
| _file_ <filepath> | Inject commands line-by-line from a file  |
| _run_ <args> |  Run a python3 script from a file: This is passed to the python3 interpreter so this way you can also pass arguments to the script itself |
| _environment_ | Inject the list of environment variables from the [environment] section |
| _sttysize_ | Inject the size of the current terminal using the Linux stty command |
| _help_ | Toggle the help menu |
| _break_ | Send a serial break |
| _stop_ | Stop the currently running script |
| _nop_ | No operation (a placeholder) |


# Scripting with Python3

If you are familiar with python you should not have problems in using the
scripting facility of `termswx`.

A `termswx.py` helper script and a `test.py` script has been provided to help
get you started with your own scripts.

The `termswx.py` helper script has the following mixin classes that you can use:

## class LoggerMixin

Provides logging in a named logfile.

### Constructor

Allows you to specify the name of the file used for logging.

### cmd method

When you use the `cmd` method, the command and its response will automatically
be logged in the file.

### add_log method

Individual commands, text, or responses can be added using the `add_log` method.

### save method

Save all the collected log information in named file.

Remember to call `save` to save the content of the file to disk before your
script exits.

It is possible to setup a `sigint` signal handler that calls `save` so you get
your logs even when you cancel the script.

## class LinuxLoginMixin

### login method

This mixin provides a login method that takes a username and password and tries
to login to a Linux based system.

## class MenuMixin

### show_menu method

This provides a `show_menu` method that takes a menu, prompt string and a title
string and shows this menu on the terminal and waits for user input (default
wait time is 60s).

When the user selects one of the menu items by entering its number and hitting
the enter key, the script run the command that was provide in the menu.

### menu structure

The menu structure looks like this:

    menu = (
           ('Menu item title', method_to_call),
           ...,
    )

You can see an example in the `test.py` file.

## class TerminalIo

This is the base class used by the mixin classes, but you can use it directly if
you do not plan to use any of the mixins.

It has the following support:

### command method

Send a command and wait for the response

### read_response method

Captures a command response which may consist of any lines

### Constructor

Allows you to provide a prompt as a regular expression and a timeout. The other
methods will use prompt to delimit responses and timeout if the expected prompt
is not received within the time limit.

There are more fine-grained control and methods that you can use, but these are
the basic methods.

# Design of TermSWX

You can read about the design [here](/design.md).

# Acknowledgements

The Terminal Switch feature set was heavily inspired by the excellent
[TermHub](https://github.com/allannielsen/termhub) program.

[modeline]: # ( vim: set ts=4 sw=4 sts=4 tw=80 cc=80 et ft=markdown : )
