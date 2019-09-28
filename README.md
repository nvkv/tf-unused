# tf-unused

[![Build Status](https://travis-ci.org/nvkv/tf-unused.svg?branch=master)](https://travis-ci.org/nvkv/tf-unused)

Find unused variables in terraform module.

## Installation

- Download binary for appropriate platform from [releases](https://github.com/nvkv/tf-unused/releases) page.
- Put it somewhere in `$PATH`

## Usage

`tf-unused <path-to-tf-module>`

When no path specified, `tf-unused` will check current directory instead.
If there are unused variables, they will be printed out and process will return non-zero return code.
Otherwise nothing will be printed and process will exit with 0.

## Examples

```
% tf-unused tests/fixtures/has_unused/
In tests/fixtures/has_unused/vars.tf:
 * Unused definition legacy_switch_i_forgot_to_remove
 * Unused definition surprisingly_unimportant_variable

In tests/fixtures/has_unused/some.tfvars:
 * Unused value for some_random_variable

% echo $?
1
```

```
% tf-unused tests/fixtures/has_no_unused/

% echo $?
0
```

