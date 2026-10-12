# Throwaway spike: prove Qt6 + QML opens a window on this NixOS box and that
# libmpv is linkable. DELETE once Phase 1 lands for real.
cmake_minimum_required(VERSION 3.21)
project(kitsune_spike LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick)

add_executable(kitsune_spike main.cpp)
target_link_libraries(kitsune_spike PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick)

# Load Main.qml from disk rather than compiling a QML module: the spike only
# needs to prove a window opens, not to exercise the resource system.
target_compile_definitions(kitsune_spike PRIVATE
    SPIKE_QML_PATH="${CMAKE_CURRENT_SOURCE_DIR}/Main.qml")

# Link libmpv if present, so the spike also proves the render API headers are
# reachable. Guarded so the window test still builds on a box without libmpv.
find_package(PkgConfig REQUIRED)
pkg_check_modules(MPV IMPORTED_TARGET mpv)
if (MPV_FOUND)
    target_link_libraries(kitsune_spike PRIVATE PkgConfig::MPV)
    target_compile_definitions(kitsune_spike PRIVATE HAVE_LIBMPV=1)
endif()

install(TARGETS kitsune_spike RUNTIME DESTINATION bin)