# Binary Ninja Pattern Scanner and Generator

## Installation

### 1. Install nightly rust

https://rustup.rs/

```
  rustup install nightly
```

### 2. Build

```
  cargo build --release
```

### 3. Link to binja plugin folder

#### Linux
```sh
  ln -s ${PWD}/target/release/libbinja_patterns.so ~/.binaryninja/plugins/
```

#### Windows
##### CMD
```cmd
  mklink %APPDATA%\\Binary\ Ninja\\plugins\\binja_patterns.dll %CD%\\target\\release\\binja_patterns.dll
```
##### POWERSHELL
```ps1
  New-Item -ItemType SymbolicLink -Path "$env:APPDATA\Binary\ Ninja\plugins\binja_patterns.dll" -Target "$PWD\target\release\binja_patterns.dll"
```
