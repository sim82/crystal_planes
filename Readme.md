Rendering levels in the style of the 1983 arcade classic 'Crystal Castles' with 'kind-of' global illumination.

Global illumination uses brute-force radiosity energy transfer between planes on the cpu, exploiting embarrasing parallelism (with rayon) well as data-parallelism (with packed_simd).
I wrote the initial version years ago in c++ as an excuse to waste cpu-cycles and do some manual SIMD coding and also did versions for vulkano and amethyst along the way. 

The bevy experience was the most fun so far, hands down...

![Screenshot](/doc/screenshot1.png?raw=true "Screenshot")