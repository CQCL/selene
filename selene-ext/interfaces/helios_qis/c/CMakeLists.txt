cmake_minimum_required(VERSION 3.12)

project(HeliosSeleneInterface LANGUAGES C)

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

find_package(Selene REQUIRED)
get_target_property(selene_include_dirs Selene::Selene INTERFACE_INCLUDE_DIRECTORIES)

set(base_name "helios_selene_interface")

function(build_helios_interface log_level suffix)

    set(target_name "${base_name}${suffix}")

    add_library(${target_name} STATIC src/interface.c)
    set_target_properties(${target_name} PROPERTIES
        POSITION_INDEPENDENT_CODE ON
        C_STANDARD 11
    )
    target_compile_definitions(${target_name} PRIVATE SELENE_LOG_LEVEL=${log_level})
    target_include_directories(${target_name} PRIVATE "${selene_include_dirs}")
    target_include_directories(${target_name} PUBLIC
        "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"
        "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
    )
    if(MSVC)
        target_compile_options(${target_name} PRIVATE /TC /Zl) # compile as C, not C++
    endif()

    install(TARGETS ${target_name}
            EXPORT HeliosSeleneInterface
            ARCHIVE DESTINATION lib
            INCLUDES DESTINATION include
    )

endfunction()

build_helios_interface(0 "")
build_helios_interface(1 "_debug")
build_helios_interface(2 "_diagnostic")

install(DIRECTORY include/helios_qis
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)
install(
    EXPORT HeliosSeleneInterface
    FILE HeliosSeleneInterfaceTargets.cmake
    NAMESPACE HeliosSeleneInterface::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/HeliosSeleneInterface
)
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/HeliosSeleneInterfaceConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/HeliosSeleneInterfaceConfig.cmake"
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/HeliosSeleneInterface
)
install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/HeliosSeleneInterfaceConfig.cmake"
    DESTINATION
        ${CMAKE_INSTALL_LIBDIR}/cmake/HeliosSeleneInterface
)
