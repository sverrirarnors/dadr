!!! warning
    **dadrock** is under heavy development — pre-alpha release. Expect breaking changes.

<img src="../assets/dadrock_logo.svg" alt="dadrock" style="display:block; margin: 0 auto 2rem; max-width: 22rem;">

**DADR** is the standard, the decision record format, and the project vocabulary. 

**dadrock** is a Rust tool that implements DADR. It offers convenience functions to preview, create, edit, manage, and validate decision records. It can also direct LLM agents to execute a variety of tasks related to DADR.

## Install

Prebuilt binaries are published with each [release](https://github.com/vincentarelbundock/dadr/releases). A single-line installer script is available for macOS, Linux, and Windows. From source:

```bash
cargo install dadrock
```

Confirm the binary is on `PATH`:

```bash
dadrock --version
```

