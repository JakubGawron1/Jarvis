import 'dart:ffi';
import 'dart:io';

/// Loads `libjarvis_ffi` produced by `cargo build -p jarvis-ffi`.
/// `pull_core` replaces this library and the app restarts the isolate.
DynamicLibrary loadJarvisCore() {
  if (Platform.isWindows) {
    return DynamicLibrary.open('jarvis_ffi.dll');
  }
  if (Platform.isAndroid || Platform.isLinux) {
    return DynamicLibrary.open('libjarvis_ffi.so');
  }
  throw UnsupportedError(Platform.operatingSystem);
}
