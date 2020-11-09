Rendering levels in the style of the 1983 arcade classic 'Crystal Castles' with 'kind-of' global illumination.
Global illumination uses brute-force radiosity energy transfer between planes on the cpu, exploiting embarrasing parallelism (with rayon) well as data-parallelism (with packed_simd).
