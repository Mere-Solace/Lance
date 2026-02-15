@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
set CMAKE_GENERATOR=Ninja
set CMAKE_POLICY_VERSION_MINIMUM=3.5
set PATH=%PATH%;C:\Users\btm74.000\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja
cd /d C:\Users\btm74.000\Projects\Lance
cargo build %* 2>&1
