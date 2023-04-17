# precipice

A command-line and GUI benchmarking tool and graph generator.

## Features

 - Providing statistics (eg. min, max, standard deviation) across a custom number of runs.
 - Support for a warmup run.
 - Export a set number of runs to CSV.
 - GUI runner with file selection and command-line arguments able to be reorganised.
 - Exporter from CSV to CSV/HTML using [plotly](https://plotly.com/javascript/).
 
 
## Usage
The program is launched using the CLI with 4 main programs that can be launched:
|Command-Line Name|Function|
|--|--|
|exporter-gui|Deals with exporting in a GUI program|
|runner-gui|Deals with benching in a GUI program|
|exporter-cli|Deals with exporting in a CLI interface|
|runner-cli|Deals with benching in a CLI interface|

The GUI programs all require active user input, and the CLI programs all require no user input and can be used in scripting programs.

### Runner CLI
The runner can do a basic CLI benchmark just using:
```sh
precipice runner-cli -b "program"
```
And if you need CLI args, they all get added in one object
```sh
precipice runner-cli -b "echo" -c "'hello world'"
```

You can do a custom number of runs using
```sh
precipice runner-cli -b "program" -r 2500
```

The default trace name and file name are just the binary with the runs, and that can be customised further with the options available at `--help`.

### Exporter CLI
TODO
