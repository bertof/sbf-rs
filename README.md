# Spatial Bloom Filter
[![pipeline status](https://gitlab.com/bertof/sbf-rs/badges/master/pipeline.svg)](https://gitlab.com/bertof/sbf-rs/commits/master)
[![coverage report](https://gitlab.com/bertof/sbf-rs/badges/master/coverage.svg)](https://gitlab.com/bertof/sbf-rs/commits/master)

SBF is a probabilistic data structure
that maps elements of a space to indexed disjoint subsets of that space.

This is a reimplementation of the [C library](https://github.com/spatialbloomfilter/libSBF-cpp) by the original research group.

This repository is mirrored in [GitLab](https://gitlab.com/bertof/sbf-rs) and [Github](https://github.com/bertof/sbf-rs)

## Crate features

This crate allows the following features:

- `md4_hash` Allows to use a md4 based hashing algorithm;
- `md5_hash` Allows to use a md5 based hashing algorithm;
- `metrics` Generates and updates an internal metrics object, useful in simulations and benchmarks of the library.

By default only `md5_hash` is enabled.
