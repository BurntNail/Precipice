# precipice

A command-line and GUI benchmarking tool and graph generator.

![LOCs](https://tokei.rs/b1/github/BurntNail/Precipice)

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
The exporter can export from any number of files, by adding more `-i` flags - for example:
```sh
precipice exporter-cli -i "bench_1.csv" -i "bench_2.csv"
```

By default, the file name to export to is `precipice_bench` and it exports to a [plotly.js](https://plotly.com/javascript/) document, but you can change this:
```sh
precipice exporter-cli -i "bench_1.csv" -o "bench_1" # exports to bench_1.html
precipice exporter-cli -i "bench_1.csv" -i "bench_2.csv" -t csv # exports to precipice_bench.csv
```
The main utility in exporting to CSV is getting multiple traces into one file.


## Installation
I don't have any installers, and this isn't on any package managers right now. The only way to run it is to get the binary from the releases page and get it into your path, or to download it using the rust dependency manager:
```sh
cargo install precipice
```
