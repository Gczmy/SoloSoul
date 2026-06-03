# Project-level configuration.
cmake_minimum_required(VERSION 3.14)
project(solosoul_flutter LANGUAGES CXX)

# The name of the executable created for the application. Change this to change
# the on-disk name of your application.
set(BINARY_NAME "solosoul_flutter")

# =============================================================================
# Windows 平台: 自动修复 flutter 生成的插件注册文件
# flutter_tools 会在构建时重新生成这些文件，覆盖手动修改。
# 因此在 CMake 配置阶段自动移除有兼容性问题的插件引用。
# =============================================================================
set(_GEN_PLUGINS_FILE "${CMAKE_CURRENT_SOURCE_DIR}/flutter/generated_plugins.cmake")
if(EXISTS ${_GEN_PLUGINS_FILE})
  file(READ ${_GEN_PLUGINS_FILE} _GEN_PLUGINS_CONTENT)
  string(REPLACE "  local_auth_windows\n" "" _GEN_PLUGINS_CONTENT "${_GEN_PLUGINS_CONTENT}")
  string(REPLACE "  pdfx\n" "" _GEN_PLUGINS_CONTENT "${_GEN_PLUGINS_CONTENT}")
  file(WRITE ${_GEN_PLUGINS_FILE} "${_GEN_PLUGINS_CONTENT}")
endif()

set(_GEN_REG_FILE "${CMAKE_CURRENT_SOURCE_DIR}/flutter/generated_plugin_registrant.cc")
if(EXISTS ${_GEN_REG_FILE})
  file(READ ${_GEN_REG_FILE} _GEN_REG_CONTENT)
  string(REPLACE "#include <local_auth_windows/local_auth_plugin.h>\n" "" _GEN_REG_CONTENT "${_GEN_REG_CONTENT}")
  string(REPLACE "#include <pdfx/pdfx_plugin.h>\n" "" _GEN_REG_CONTENT "${_GEN_REG_CONTENT}")
  string(REPLACE "  LocalAuthPluginRegisterWithRegistrar(\n      registry->GetRegistrarForPlugin(\"LocalAuthPlugin\"));\n" "" _GEN_REG_CONTENT "${_GEN_REG_CONTENT}")
  string(REPLACE "  PdfxPluginRegisterWithRegistrar(\n      registry->GetRegistrarForPlugin(\"PdfxPlugin\"));\n" "" _GEN_REG_CONTENT "${_GEN_REG_CONTENT}")
  file(WRITE ${_GEN_REG_FILE} "${_GEN_REG_CONTENT}")
endif()

# Explicitly opt in to modern CMake behaviors to avoid warnings with recent
# versions of CMake.
cmake_policy(VERSION 3.14...3.25)

# Define build configuration option.
get_property(IS_MULTICONFIG GLOBAL PROPERTY GENERATOR_IS_MULTI_CONFIG)
if(IS_MULTICONFIG)
  set(CMAKE_CONFIGURATION_TYPES "Debug;Profile;Release"
    CACHE STRING "" FORCE)
else()
  if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE "Debug" CACHE
      STRING "Flutter build mode" FORCE)
    set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS
      "Debug" "Profile" "Release")
  endif()
endif()
# Define settings for the Profile build mode.
set(CMAKE_EXE_LINKER_FLAGS_PROFILE "${CMAKE_EXE_LINKER_FLAGS_RELEASE}")
set(CMAKE_SHARED_LINKER_FLAGS_PROFILE "${CMAKE_SHARED_LINKER_FLAGS_RELEASE}")
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_RELEASE}")
set(CMAKE_CXX_FLAGS_PROFILE "${CMAKE_CXX_FLAGS_RELEASE}")

# Use Unicode for all projects.
add_definitions(-DUNICODE -D_UNICODE)

# Compilation settings that should be applied to most targets.
#
# Be cautious about adding new options here, as plugins use this function by
# default. In most cases, you should add new options to specific targets instead
# of modifying this function.
function(APPLY_STANDARD_SETTINGS TARGET)
  target_compile_features(${TARGET} PUBLIC cxx_std_17)
  target_compile_options(${TARGET} PRIVATE /W4 /WX /wd"4100")
  target_compile_options(${TARGET} PRIVATE /EHsc)
  target_compile_definitions(${TARGET} PRIVATE "_HAS_EXCEPTIONS=0")
  target_compile_definitions(${TARGET} PRIVATE "$<$<CONFIG:Debug>:_DEBUG>")
endfunction()

# Flutter library and tool build rules.
set(FLUTTER_MANAGED_DIR "${CMAKE_CURRENT_SOURCE_DIR}/flutter")
add_subdirectory(${FLUTTER_MANAGED_DIR})

# Rust library build
set(RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../native")
set(RUST_TARGET "x86_64-pc-windows-msvc")

# Windows: 始终使用 Release 构建 Rust，避免 ort 预编译库的 CRT 冲突
# ort 2.0.0-rc.12 预编译二进制使用 /MD (Release CRT)，与 Flutter Debug 的 /MDd 冲突
set(RUST_BUILD_FLAG "--release")
set(RUST_OUTPUT_DIR "${RUST_DIR}/target/${RUST_TARGET}/release")

set(RUST_DLL "${RUST_OUTPUT_DIR}/solosoul_core.dll")
set(RUST_IMPLIB "${RUST_OUTPUT_DIR}/solosoul_core.lib")

add_custom_command(
  OUTPUT ${RUST_DLL} ${RUST_IMPLIB}
  COMMAND cargo build --target ${RUST_TARGET} --features sandbox ${RUST_BUILD_FLAG}
  WORKING_DIRECTORY ${RUST_DIR}
  COMMENT "Building Rust core library for Windows..."
  BYPRODUCTS ${RUST_IMPLIB}
)

add_custom_target(rust_build DEPENDS ${RUST_DLL} ${RUST_IMPLIB})

# Application build; see runner/CMakeLists.txt.
add_subdirectory("runner")


# Generated plugin build rules, which manage building the plugins and adding
# them to the application.

# Windows 平台: 移除有兼容性问题的插件
# pdfx: CMake 4.x 不兼容其旧版 DownloadProject.cmake
# local_auth_windows: VS 2026 已移除 <experimental/coroutine>
list(REMOVE_ITEM FLUTTER_PLUGIN_LIST pdfx)
list(REMOVE_ITEM FLUTTER_PLUGIN_LIST local_auth_windows)

include(flutter/generated_plugins.cmake)


# === Installation ===
# Support files are copied into place next to the executable, so that it can
# run in place. This is done instead of making a separate bundle (as on Linux)
# so that building and running from within Visual Studio will work.
set(BUILD_BUNDLE_DIR "$<TARGET_FILE_DIR:${BINARY_NAME}>")
# Make the "install" step default, as it's required to run.
set(CMAKE_VS_INCLUDE_INSTALL_TO_DEFAULT_BUILD 1)
if(CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
  set(CMAKE_INSTALL_PREFIX "${BUILD_BUNDLE_DIR}" CACHE PATH "..." FORCE)
endif()

set(INSTALL_BUNDLE_DATA_DIR "${CMAKE_INSTALL_PREFIX}/data")
set(INSTALL_BUNDLE_LIB_DIR "${CMAKE_INSTALL_PREFIX}")

install(TARGETS ${BINARY_NAME} RUNTIME DESTINATION "${CMAKE_INSTALL_PREFIX}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_ICU_DATA_FILE}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  COMPONENT Runtime)

if(PLUGIN_BUNDLED_LIBRARIES)
  install(FILES "${PLUGIN_BUNDLED_LIBRARIES}"
    DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
    COMPONENT Runtime)
endif()

# Copy the native assets provided by the build.dart from all packages.
set(NATIVE_ASSETS_DIR "${PROJECT_BUILD_DIR}native_assets/windows/")
install(DIRECTORY "${NATIVE_ASSETS_DIR}"
   DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
   COMPONENT Runtime)

# Fully re-copy the assets directory on each build to avoid having stale files
# from a previous install.
set(FLUTTER_ASSET_DIR_NAME "flutter_assets")
install(CODE "
  file(REMOVE_RECURSE \"${INSTALL_BUNDLE_DATA_DIR}/${FLUTTER_ASSET_DIR_NAME}\")
  " COMPONENT Runtime)
install(DIRECTORY "${PROJECT_BUILD_DIR}/${FLUTTER_ASSET_DIR_NAME}"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}" COMPONENT Runtime)

# Install the AOT library on non-Debug builds only.
install(FILES "${AOT_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  CONFIGURATIONS Profile;Release
  COMPONENT Runtime)
