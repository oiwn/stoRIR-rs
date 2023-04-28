# stoRIR-rs

Rust implementation of Stochastic Room Impulse Response Generation.

````
Usage: storir [OPTIONS]

Options:
  -a, --algo <ALGO>                  Algo [default: simple]
  -s, --sample-rate <SAMPLE_RATE>    Sample rate [default: 44100]
  -f, --folder <FOLDER>              Folder to store wav files [default: impulses]
  -n, --num-impulses <NUM_IMPULSES>  Number of impulses to generate [default: 5]
      --rt60 <RT60>                  Reverberation time in [ms] [default: 500]
      --edt <EDT>                    Early decay time [ms] [default: 50]
      --itdg <ITDG>                  Initial time delay gap [ms] [default: 4]
      --er-duration <ER_DURATION>    Early reflections duration [ms] [default: 100]
  -h, --help                         Print help
  -V, --version                      Print version
```