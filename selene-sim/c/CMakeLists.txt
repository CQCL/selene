cmake_minimum_required(VERSION 3.12)
project(Selene LANGUAGES C)

include(GNUInstallDirs)

# Define a regular interface target (not imported!)
add_library(Selene INTERFACE)

# Absolute path to Rust-built library
set(selene_lib "${CMAKE_INSTALL_PREFIX}/lib/libselene.so")

# Populate properties
target_include_directories(Selene INTERFACE
    "${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_INCLUDEDIR}"
)
target_link_libraries(Selene INTERFACE
    "${selene_lib}"
)

# Install headers
install(DIRECTORY include/
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Install the interface target (this now works!)
install(
    TARGETS Selene
    EXPORT SeleneTargets
)

install(
    EXPORT SeleneTargets
    FILE SeleneTargets.cmake
    NAMESPACE Selene::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/Selene
)

# Config file
include(CMakePackageConfigHelpers)
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SeleneConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/SeleneConfig.cmake"
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/Selene
)
install(
    FILES "${CMAKE_CURRENT_BINARY_DIR}/SeleneConfig.cmake"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/Selene
)
