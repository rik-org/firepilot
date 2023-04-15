# Firepilot 

`firepilot` is a rust library to interact with [firecracker](firecracker), it
can be used to configure and run firecracker micro VMs. It relies on
auto-generated models provided by the [project's OpenAPI](firecracker-openapi),
those models are available in the dependency `firecracker-models`.

There are some Firecracker features that are not yet supported. If you need one
of them, please open an issue.

This crate is inspired by
[firecracker-go-sdk](https://github.com/firecracker-microvm/firecracker-go-sdk)
a Go SDK for Firecracker.

## Design

Our main goal is to provide an opinionated way to interact and manage
firecracker microVMs, for our bigger project [rik](rik). However, we wanted to
make this library available for everyone, with an unopinionated way to manage
VMs. To do so, this crate contains two way to create VMs:

- Using high-level [Machine] abstraction: through simple methods, you can create
  and control the lifecyle of a VM. This is the recommended way to use this
  crate.
- Using low-level [Executor]: you can fully control and manage each step of the
  VM lifecycle. This is useful if you want to have more control over the VM
  configuration and not satisfied with the current high-level abstraction.

## Examples

You can find full examples in the [`examples`][firepilot-examples] directory.
Examples are auto-sufficent, they will download a sample rootfs and kernel
provided by Firecracker, but you must have firecracker installed on your system.

### MSRV

The minimum supported rust version is `1.60.0`.

[firecracker]: https://github.com/firecracker-microvm/firecracker/
[firecracker-openapi]: https://github.com/firecracker-microvm/firecracker/blob/main/src/api_server/swagger/firecracker.yaml
[rik]: https://github.com/rik-org/rik
[firepilot-examples]: https://github.com/rik-org/firepilot/tree/main/examples