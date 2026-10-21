cmake_minimum_required(VERSION 3.14)
set(PROJECT_NAME "history_core")
project(${PROJECT_NAME} LANGUAGES CXX)

# 엔진 DLL은 tools/build-native.ps1이 미리 지어 둔다(Rust). 여기서는 앱 묶음에 넣기만 한다.
set(history_core_bundled_libraries
  "${CMAKE_CURRENT_SOURCE_DIR}/../native/windows-x64/history_core.dll"
  PARENT_SCOPE
)
