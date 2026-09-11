cmake_minimum_required(VERSION 3.22)
project(OmaStore VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)
include(CTest)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

find_package(Qt6 6.4 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2)
find_program(CARGO_EXECUTABLE cargo REQUIRED)
qt_standard_project_setup()

file(GLOB_RECURSE RUST_SOURCES CONFIGURE_DEPENDS "${CMAKE_SOURCE_DIR}/native/core/src/*.rs" "${CMAKE_SOURCE_DIR}/crates/catalogue/src/*.rs")
set(CORE_BINARY "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-core")
add_custom_command(
    OUTPUT "${CORE_BINARY}"
    COMMAND "${CARGO_EXECUTABLE}" build --locked --release
        --manifest-path "${CMAKE_SOURCE_DIR}/Cargo.toml"
        --target-dir "${CMAKE_BINARY_DIR}/cargo"
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${CMAKE_BINARY_DIR}/cargo/release/omastore-core" "${CORE_BINARY}"
    DEPENDS Cargo.toml Cargo.lock native/core/Cargo.toml crates/catalogue/Cargo.toml data/registry.json ${RUST_SOURCES}
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    VERBATIM
)
add_custom_target(omastore-core ALL DEPENDS "${CORE_BINARY}")

qt_add_executable(omastore
    native/ui/main.cpp
    native/ui/CoreBridge.cpp
    native/ui/CoreBridge.h
)
set_source_files_properties(native/ui/qml/Main.qml PROPERTIES QT_RESOURCE_ALIAS Main.qml)
qt_add_qml_module(omastore
    URI OmaStore
    VERSION 1.0
    RESOURCE_PREFIX /qt/qml
    QML_FILES native/ui/qml/Main.qml
)
target_link_libraries(omastore PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
)
add_dependencies(omastore omastore-core)

install(TARGETS omastore RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(PROGRAMS "${CORE_BINARY}" DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(FILES packaging/io.github.tcballard.OmaStore.desktop
    DESTINATION "${CMAKE_INSTALL_DATADIR}/applications")

if(BUILD_TESTING)
    find_package(Qt6 6.4 REQUIRED COMPONENTS Test)
    target_compile_definitions(omastore PRIVATE OMASTORE_TESTING)
    target_link_libraries(omastore PRIVATE Qt6::Test)
    find_package(Python3 3.10 REQUIRED COMPONENTS Interpreter)
    add_test(NAME core-process-contract
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/core_process.py" "${CORE_BINARY}")
    add_test(NAME native-discovery
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/native_discovery.py" "$<TARGET_FILE:omastore>")
    set_tests_properties(native-discovery PROPERTIES TIMEOUT 30)
    add_test(NAME native-startup COMMAND omastore --smoke-test)
    set_tests_properties(core-process-contract native-startup PROPERTIES TIMEOUT 20)
    set_tests_properties(native-startup PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
endif()
