# Fuzzy

Fuzzer written in rust to explore security, rust, and everything in between.

## Dictionary

Parts of dictionary were politely cloned from the [big list of naughty strings](https://github.com/minimaxir/big-list-of-naughty-strings/).
The rest is a combination of things that I've explored in the past.

## Todo

A little todo list

- generate uuid for each request and pass it down as context for metrics
- config option to add a requests uuid as a header to the request made for tracing
- fuzz body parameters
- fuzz headers
- support for all http methods
- multiple fuzz for a single value, e.g. userId={fuzz} = userId={fuzz1}{fuzz2} etc
- match responses based on header values
- match responses based on body
- allow response body to have wildcards
- allow response body value to match the fuzzed value (see reflected output, make this inverted for any other behavior)