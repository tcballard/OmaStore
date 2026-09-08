cmake_minimum_required(VERSION 3.22)
project(OmaStore VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)
include(CTest)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

find_package(Qt6 6.4 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 DBus Network)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
endif()
find_program(CARGO_EXECUTABLE cargo REQUIRED)
qt_standard_project_setup()

set(CORE_BINARY "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-core")
set(OMASTORE_CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo" CACHE PATH "Cargo build cache directory")
option(OMASTORE_DEVELOPMENT_DATA "Embed explicitly synthetic development listings" OFF)
set(CARGO_FEATURE_ARGS "")
if(OMASTORE_DEVELOPMENT_DATA)
    list(APPEND CARGO_FEATURE_ARGS --features omastore-core/development-catalogue,omastore-service/development-catalogue,omastore-service/development-workflow)
endif()
# Invoke Cargo each time: it owns incremental tracking for every Rust module.
add_custom_target(omastore-core ALL
    COMMAND "${CARGO_EXECUTABLE}" build --locked --release
        --manifest-path "${CMAKE_SOURCE_DIR}/Cargo.toml"
        --target-dir "${OMASTORE_CARGO_TARGET_DIR}"
        ${CARGO_FEATURE_ARGS}
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${OMASTORE_CARGO_TARGET_DIR}/release/omastore-core" "${CORE_BINARY}"
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${OMASTORE_CARGO_TARGET_DIR}/release/omastore-service" "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-service"
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${OMASTORE_CARGO_TARGET_DIR}/release/omastore-validate" "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-validate"
    BYPRODUCTS "${CORE_BINARY}"
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    VERBATIM
)

qt_add_executable(omastore
    native/ui/main.cpp
    native/ui/CoreBridge.cpp
    native/ui/CoreBridge.h
    native/ui/DesktopSettings.cpp
    native/ui/DesktopSettings.h
    native/ui/MediaPreview.cpp
    native/ui/MediaPreview.h
    native/ui/Preparation.cpp
    native/ui/Preparation.h
)
set_source_files_properties(native/ui/qml/PlanDialog.qml PROPERTIES QT_RESOURCE_ALIAS PlanDialog.qml)
set_source_files_properties(native/ui/qml/SetupsPage.qml PROPERTIES QT_RESOURCE_ALIAS SetupsPage.qml)
set_source_files_properties(native/ui/qml/CataloguePicker.qml PROPERTIES QT_RESOURCE_ALIAS CataloguePicker.qml)
set_source_files_properties(native/ui/qml/MakersPage.qml PROPERTIES QT_RESOURCE_ALIAS MakersPage.qml)
set_source_files_properties(native/ui/qml/EditorialPanel.qml PROPERTIES QT_RESOURCE_ALIAS EditorialPanel.qml)
set_source_files_properties(native/ui/qml/Main.qml PROPERTIES QT_RESOURCE_ALIAS Main.qml)
set_source_files_properties(native/ui/qml/AppCard.qml PROPERTIES QT_RESOURCE_ALIAS AppCard.qml)
set_source_files_properties(native/ui/qml/DetailPage.qml PROPERTIES QT_RESOURCE_ALIAS DetailPage.qml)
set_source_files_properties(native/ui/qml/SubmitPage.qml PROPERTIES QT_RESOURCE_ALIAS SubmitPage.qml)
set_source_files_properties(native/ui/qml/WorkspacePage.qml PROPERTIES QT_RESOURCE_ALIAS WorkspacePage.qml)
set_source_files_properties(native/ui/qml/ReviewPanel.qml PROPERTIES QT_RESOURCE_ALIAS ReviewPanel.qml)
set_source_files_properties(native/ui/qml/DraftPanel.qml PROPERTIES QT_RESOURCE_ALIAS DraftPanel.qml)
set_source_files_properties(native/ui/qml/ValueEditor.qml PROPERTIES QT_RESOURCE_ALIAS ValueEditor.qml)
qt_add_qml_module(omastore
    URI OmaStore
    VERSION 1.0
    RESOURCE_PREFIX /qt/qml
    QML_FILES native/ui/qml/PlanDialog.qml native/ui/qml/SetupsPage.qml native/ui/qml/CataloguePicker.qml native/ui/qml/MakersPage.qml native/ui/qml/EditorialPanel.qml native/ui/qml/MonitoringPanel.qml native/ui/qml/ReviewPanel.qml native/ui/qml/DraftPanel.qml native/ui/qml/ValueEditor.qml native/ui/qml/Main.qml native/ui/qml/AppCard.qml native/ui/qml/DetailPage.qml native/ui/qml/SubmitPage.qml native/ui/qml/WorkspacePage.qml
)
target_link_libraries(omastore PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2 Qt6::DBus Qt6::Network
)
if(BUILD_TESTING)
    target_link_libraries(omastore PRIVATE Qt6::Test)
    target_compile_definitions(omastore PRIVATE OMASTORE_QA)
endif()
add_dependencies(omastore omastore-core)
if(OMASTORE_DEVELOPMENT_DATA)
    target_compile_definitions(omastore PRIVATE OMASTORE_DEVELOPMENT_DATA)
endif()

install(TARGETS omastore RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(PROGRAMS "${CORE_BINARY}" DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(FILES packaging/io.github.tcballard.OmaStore.desktop
    DESTINATION "${CMAKE_INSTALL_DATADIR}/applications")
install(FILES packaging/io.github.tcballard.OmaStore.metainfo.xml
    DESTINATION "${CMAKE_INSTALL_DATADIR}/metainfo")
install(FILES LICENSE DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/omastore")

if(BUILD_TESTING)
    qt_add_executable(native-contracts tests/native_contracts.cpp native/ui/MediaPreview.cpp native/ui/MediaPreview.h native/ui/Preparation.cpp native/ui/Preparation.h)
    target_include_directories(native-contracts PRIVATE native/ui)
    target_link_libraries(native-contracts PRIVATE Qt6::Core Qt6::Gui Qt6::Network Qt6::Test)
    add_test(NAME native-contracts COMMAND native-contracts)
    find_package(Python3 3.10 REQUIRED COMPONENTS Interpreter)
    add_test(NAME core-process-contract
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/core_process.py" "${CORE_BINARY}")
    add_test(NAME native-startup COMMAND omastore --smoke-test)
    if(OMASTORE_DEVELOPMENT_DATA)
        foreach(SIZE IN ITEMS 800x600 1280x800 1920x1080)
            add_test(NAME native-interaction-${SIZE} COMMAND omastore --demo --ui-test --window-size ${SIZE})
            set_tests_properties(native-interaction-${SIZE} PROPERTIES TIMEOUT 20
                ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
        endforeach()
        add_test(NAME native-interaction-200-percent COMMAND omastore --demo --ui-test --window-size 800x600)
        set_tests_properties(native-interaction-200-percent PROPERTIES TIMEOUT 20
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QT_SCALE_FACTOR=2")
        add_test(NAME catalogue-http COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/http_process.py"
            "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-service" "${CORE_BINARY}" "${CMAKE_SOURCE_DIR}/tests/fixtures/catalogue.json" --demo)
    else()
        add_test(NAME production-fixture-exclusion COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/fixture_exclusion.py" "${CORE_BINARY}")
        add_test(NAME catalogue-http COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/http_process.py"
            "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-service" "${CORE_BINARY}" "${CMAKE_SOURCE_DIR}/data/registry.json")
    endif()
    set_tests_properties(catalogue-http PROPERTIES TIMEOUT 30 SKIP_RETURN_CODE 77)
    set_tests_properties(core-process-contract native-startup PROPERTIES TIMEOUT 20)
    set_tests_properties(native-startup PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
endif()
