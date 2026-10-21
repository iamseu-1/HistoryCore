import 'dart:io';

/// `tools/build-native.ps1`이 놓은 DLL(또는 HISTORY_CORE_LIBRARY).
String builtLibrary() =>
    Platform.environment['HISTORY_CORE_LIBRARY'] ??
    [Directory.current.path, 'native', 'windows-x64', 'history_core.dll'].join(Platform.pathSeparator);

/// 미리 지은 DLL로 도는 검사는 Windows에서만.
final Object windowsOnly = Platform.isWindows ? false : 'Windows에서만 — 미리 지은 DLL로 돈다';
