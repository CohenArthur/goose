# goose

A micro Kernel written in Rust. The goal is to have a small number of syscall 
with a strong emphasis on IPC

## Roadmap

- [ ] Virtual Memory Manager (in progress)
- [ ] Basic in-kernel filesystem
- [ ] Device-tree handling
- [x] In-kernel ELF loader
- [ ] Userland process (in progress)
- [ ] IPC implementation
- [ ] Drivers
    - [ ] Driver API
    - [ ] Kernel API

## Project structure

The project is divided in 2 components, the kernel and the drivers. Most of the 
drivers runs in userland, but some are required to run in kernel just to 
provide the basic kernel functionalities (UART, interrupt hardware, ...)
:warning: **At the moment userland drivers are not implemented**

## Try it out
Choose your desired board and `cd` to the corresponding directory:
- Qemu RISC-V (virt) --> `riscv_qemuvirt`
- Qemu AArch64 (virt) --> `aarch64_qemuvirt`

Then just run:
```console
$ cargo run
```

For qemu target it will launch Qemu. For other targets the hope is to flash them instead

### Requirement
- A rust nightly toolchain
- Clang compiler (for tests)

#### Nix
If you use Nix you can run `nix develop` to get a shell with everything needed
to test GoOSe

### Build
### Board project
Go to a board project (ex. riscv_qemuvirt) and then:
```console
$ cargo build
```

### Tests
GoOSe also comes with unit tests that run directly on hardware and output the 
result over serial. When using Qemu, you can also have an exit code != 0 on 
failure.

```console
$ cargo run -F launch_tests
```
:warning: **Tests might be slow to run as GoOSe is not really optimized. You can
append `--release` to the previous cargo command line to boost performance but 
please be aware that some test might pass in debug and not in release. Feel 
free to open an issue if you encounter such a case**
