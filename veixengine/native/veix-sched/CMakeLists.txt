# Veix 进程/线程调度：静态库，无 main，不使用 printf/malloc（HeapAlloc + Win32）。
cmake_minimum_required(VERSION 3.16)
project(veix_sched LANGUAGES C)

if(NOT WIN32)
  message(FATAL_ERROR "veix-sched Build1 当前仅支持 Windows（Win32 线程与进程 API）")
endif()

set(CMAKE_C_STANDARD 11)

add_library(veix_sched STATIC
  src/sched_win.c
  src/process_win.c
)

target_include_directories(veix_sched PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>
)

target_compile_definitions(veix_sched PRIVATE
  WIN32_LEAN_AND_MEAN
  NOMINMAX
)

if(MSVC)
  target_compile_definitions(veix_sched PRIVATE UNICODE _UNICODE _CRT_SECURE_NO_WARNINGS)
endif()
