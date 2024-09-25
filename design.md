# termswx design

The Terminal Switch consists of a number of components as shown in the diagram
below:

![Overview](/documentation/overview.png "TermSWX Overview")

The components in the above diagram is described in the following sections.

## term switch

The term switch component is at the center of the Terminal Switch application
and passes messages to and from the services in the application.

It has state that ensures that messages are passed to the correct services and
also provides functionality for the initial connection between the term switch
and a service.

![Local](/documentation/local_events.png "Local Events")

## Console Service

The console service uses the local console interface to process the user input
and show any output.

It starts two threads: one for keyboard input and one for console output.

* input thread: read key input from the local console and looks up shortcuts
and execute the actions bound to the these.  Other keys are sent via a channel
to the term switch.
* output thread: wait for keys on the channel from the term switch and print
these on the local console.

![Keyinput](/documentation/keyinput.png "Console Service key input")

The console service can show a help menu with information about the current
state of the application.  That also includes a list of key bindings.

### Script Runner

The console service has a script runner helper module that can execute python
scripts.

The script runner starts a python executable and passes data via the child
process stdin, stdout and stderr file handles.

The stderr has a extra feature: a single prefix character will trigger a script
runner action as shown in the table below:

| Prefix Byte | Usage description | Text colors |
| :----| :---- | :---- |
| 0x11 | The remaining line is an alert message | White on magenta |
| 0x12 | The remaining line is a menu title | White on green |
| 0x13 | The remaining line is a menu item | Black on yellow |
| 0x14 | The remaining line is an input prompt | Black on green |
| 0x15 | User text | White on green |
| 0x16 | Switch to binary mode |   |
| 0x17 | Switch to text mode |   |

The two binary modes are used to transfer files across the serial port.

#### ANSI Escape sequence filter

When a script is running there is a ANSI escape sequence filter applied to the
characters coming from the serial service.  This to remove the sequences that
are often sent by terminals to control text color and cursor position.

The filter is very simple but catches the typical ANSI sequences used in the
terminals that I have tested.

Here is the state diagram of the filter:

![ANSIFilter](/documentation/ansi_filter.png "ANSI Escape Sequence Filter")

The filter is designed to handle the sequences listed [here](/documentation/ansi_escape_prompt.md).

## Serial Service

The serial service starts a thread that tries to open the serial port provided
on the application command line.  If the port opens successfully then two new
threads are started:

* console input: waits for input from the term switch and write these characters
  to the serial port.
* serial input: waits for input from the serial port and sends these keys to the
  term switch.

If the port is closed there is a command line option to wait for it to reopen,
but if that is not used the thread will exit and terminate the application.

The serial service is not started if the user specifies a network destination
instead of a serial port.

## Network Service

The network service has two roles: It can allow a user to connect to a remote
serial port and it can expose a local serial port on the network and accept a
remote connection.

Here is a overview of the two parties in this scenario:

![Remote](/documentation/remote_events.png "Connecting to a remote serial port")

### Connecting to a remote serial port

If a hostname and portnumber is specified on the command line the network
services starts two threads:

* network output thread: opens a TCP connection to the destination and waits for
  characters from the term switch.  This way it takes the place of the serial
  service.  The receive part of the connection is passed to the input thread.
* network input thread: waits for input on the input part of the network
  connection.  These characters are passed to the term switch like they were
  received from a serial port.

The network connection is not started if the user specifies a serial port on the
command line.

### Exposing the local serial port on the network

If a serial port and a TCP portnumber are specified on the command line the
network service will start a server thread that listens on this TCP port.

For each client that connects the thread creates a bidirectional client channel
to the term switch and starts two threads that handles the network communication:

* client receiver: waits for characters from the term switch and writes these to
  the client network connection.
* network receiver: waits for characters from the client network connection and
  sends these to the term switch.

The term switch component registers each of the connected clients and ensures
that each of them are sent a copy of what passes between the local console and
the local serial port, and that input from the remote console gets passed to the
local serial port.

![tcpserver](/documentation/tcpserver.png "Connection to local")

When a client disconnects it is unregistered from the term switch and the client
connection is closed.

## Configuration Service

The configuration service is a set of helper functions.

It can read a configuration file `~/.config/termswx/config.toml` and configure
the application with the settings found in the file.

It also uses this to create keymap that can be used by the console service to
find keyboard shortcuts and the associated actions.

If no configuration file is found at startup a file with a default configuration
is created.

## Logger Service

The loggers service is a set of helper functions.

It provides a logger that is configured to a certain log level that is specified
on the application command line.

When clients use the debug, info, warn and error standard logging functions the
output will go to this logger instance and will be written to a logfile which by
default is `/tmp/termswx_trace.log` but you can override this with a command line
argument.

The logfile is not deleted by the application so it will grow over time.

# Application Start

The main module parses the application command line and reads the configuration
file and then starts the appropriate services.

Below is the flow of events when the application starts.

![start](/documentation/start.png "Application Start")

This diagram also shows a test feature that was later removed: The use of a test
file to simulate a real serial port device.



[modeline]: # ( vim: set ts=4 sw=4 sts=4 tw=80 cc=80 et ft=markdown : )

