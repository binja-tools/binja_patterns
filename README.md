# Binary Ninja Pattern Scanner and Generator

## Installation

### 1. Install nightly rust

https://rustup.rs/

```
  rustup install nightly
```

### 2. Update deps

If you are building for the stable release, uncomment the `branch` fields in `Cargo.toml`.

Make sure you build against the latest version of the binja api:

```sh
cargo update
```

### 3. Build

```sh
cargo build --release
```

### 4. Link to binja plugin folder

#### Linux
```sh
ln -s ${PWD}/target/release/libbinja_patterns.so ~/.binaryninja/plugins/
```

#### Windows
##### CMD
```cmd
mklink "%APPDATA%\Binary Ninja\plugins\binja_patterns.dll" "%CD%\target\release\binja_patterns.dll"
```
##### POWERSHELL
```ps1
New-Item -ItemType SymbolicLink -Path "$env:APPDATA\Binary Ninja\plugins\binja_patterns.dll" -Target "$PWD\target\release\binja_patterns.dll"
```
