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
    native/ui/Instance.cpp native/ui/Instance.h
    native/ui/DesktopSettings.cpp
    native/ui/DesktopSettings.h
    native/ui/MediaPreview.cpp
    native/ui/MediaPreview.h
    native/ui/Preparation.cpp
    native/ui/Preparation.h
)
set_source_files_properties(native/ui/qml/LibraryPage.qml PROPERTIES QT_RESOURCE_ALIAS LibraryPage.qml)
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
set_source_files_properties(native/ui/qml/RefundsPanel.qml PROPERTIES QT_RESOURCE_ALIAS RefundsPanel.qml)
set_source_files_properties(native/ui/qml/FinancesPanel.qml PROPERTIES QT_RESOURCE_ALIAS FinancesPanel.qml)
set_source_files_properties(native/ui/qml/CommerceOffersPanel.qml PROPERTIES QT_RESOURCE_ALIAS CommerceOffersPanel.qml)
set_source_files_properties(native/ui/qml/PurchasesDialog.qml PROPERTIES QT_RESOURCE_ALIAS PurchasesDialog.qml)
set_source_files_properties(native/ui/qml/CommercePanel.qml PROPERTIES QT_RESOURCE_ALIAS CommercePanel.qml)
set_source_files_properties(native/ui/qml/RemixDialog.qml PROPERTIES QT_RESOURCE_ALIAS RemixDialog.qml)
set_source_files_properties(native/ui/qml/SettingsDialog.qml PROPERTIES QT_RESOURCE_ALIAS SettingsDialog.qml)
set_source_files_properties(native/ui/qml/ReadinessPanel.qml PROPERTIES QT_RESOURCE_ALIAS ReadinessPanel.qml)
set_source_files_properties(native/ui/qml/MonitoringPanel.qml PROPERTIES QT_RESOURCE_ALIAS MonitoringPanel.qml)
set_source_files_properties(native/ui/qml/StoreTheme.qml PROPERTIES QT_RESOURCE_ALIAS StoreTheme.qml)
set_source_files_properties(native/ui/qml/ActionButton.qml PROPERTIES QT_RESOURCE_ALIAS ActionButton.qml)
set_source_files_properties(native/ui/qml/AppSymbol.qml PROPERTIES QT_RESOURCE_ALIAS AppSymbol.qml)
set_source_files_properties(native/ui/qml/DiscoveryFeature.qml PROPERTIES QT_RESOURCE_ALIAS DiscoveryFeature.qml)
set_source_files_properties(native/ui/assets/omacalc-upstream.png PROPERTIES QT_RESOURCE_ALIAS assets/omacalc-upstream.png)
qt_add_qml_module(omastore
    URI OmaStore
    VERSION 1.0
    RESOURCE_PREFIX /qt/qml
    RESOURCES native/ui/assets/omacalc-upstream.png
    QML_FILES native/ui/qml/StoreTheme.qml native/ui/qml/ActionButton.qml native/ui/qml/AppSymbol.qml native/ui/qml/DiscoveryFeature.qml native/ui/qml/RefundsPanel.qml native/ui/qml/FinancesPanel.qml native/ui/qml/CommerceOffersPanel.qml native/ui/qml/PurchasesDialog.qml native/ui/qml/CommercePanel.qml native/ui/qml/RemixDialog.qml native/ui/qml/SettingsDialog.qml native/ui/qml/ReadinessPanel.qml native/ui/qml/LibraryPage.qml native/ui/qml/PlanDialog.qml native/ui/qml/SetupsPage.qml native/ui/qml/CataloguePicker.qml native/ui/qml/MakersPage.qml native/ui/qml/EditorialPanel.qml native/ui/qml/MonitoringPanel.qml native/ui/qml/ReviewPanel.qml native/ui/qml/DraftPanel.qml native/ui/qml/ValueEditor.qml native/ui/qml/Main.qml native/ui/qml/AppCard.qml native/ui/qml/DetailPage.qml native/ui/qml/SubmitPage.qml native/ui/qml/WorkspacePage.qml
)
foreach(component IN ITEMS AppShelf AppShowcase StoreHeader)
    set_source_files_properties(native/ui/qml/${component}.qml PROPERTIES QT_RESOURCE_ALIAS ${component}.qml)
endforeach()
file(GLOB OMASTORE_DESIGN_ASSETS RELATIVE "${CMAKE_SOURCE_DIR}" native/ui/assets/fonts/*.ttf native/ui/assets/icons/*.svg)
foreach(asset IN LISTS OMASTORE_DESIGN_ASSETS)
    string(REPLACE "native/ui/" "" alias "${asset}")
    set_source_files_properties(${asset} PROPERTIES QT_RESOURCE_ALIAS "${alias}")
endforeach()
qt_target_qml_sources(omastore QML_FILES native/ui/qml/AppShelf.qml native/ui/qml/AppShowcase.qml native/ui/qml/StoreHeader.qml RESOURCES ${OMASTORE_DESIGN_ASSETS})
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
install(FILES native/ui/assets/OMACALC-LICENSE DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/omastore")
install(FILES native/ui/assets/fonts/InstrumentSerif-OFL.txt native/ui/assets/fonts/IBMPlexSans-OFL.txt DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/omastore")
install(FILES native/ui/assets/icons/LICENSE DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/omastore" RENAME Feather-LICENSE)
install(FILES LICENSE DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/omastore")

if(BUILD_TESTING)
    qt_add_executable(native-contracts tests/native_contracts.cpp native/ui/MediaPreview.cpp native/ui/MediaPreview.h native/ui/Preparation.cpp native/ui/Preparation.h)
    qt_add_executable(desktop-settings-tests tests/desktop_settings.cpp native/ui/DesktopSettings.cpp native/ui/DesktopSettings.h)
    target_include_directories(desktop-settings-tests PRIVATE native/ui)
    target_link_libraries(desktop-settings-tests PRIVATE Qt6::Core Qt6::Gui Qt6::DBus Qt6::Test)
    add_test(NAME desktop-settings COMMAND desktop-settings-tests)
    set_tests_properties(desktop-settings PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen" TIMEOUT 30)
    target_include_directories(native-contracts PRIVATE native/ui)
    target_link_libraries(native-contracts PRIVATE Qt6::Core Qt6::Gui Qt6::Network Qt6::Test)
    add_test(NAME native-contracts COMMAND native-contracts)
    find_package(Python3 3.10 REQUIRED COMPONENTS Interpreter)
    add_test(NAME core-process-contract
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/core_process.py" "${CORE_BINARY}")
    add_test(NAME native-instance COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/native_instance.py" "$<TARGET_FILE:omastore>")
    set_tests_properties(native-instance PROPERTIES TIMEOUT 30 SKIP_RETURN_CODE 77)
    add_test(NAME native-startup COMMAND omastore --smoke-test)
    if(NOT OMASTORE_DEVELOPMENT_DATA)
        add_test(NAME repository-journey COMMAND omastore --storefront-test)
        foreach(THEME IN ITEMS dark light)
            add_test(NAME repository-theme-${THEME} COMMAND omastore --storefront-test --window-size 800x600 --theme-fixture "${CMAKE_SOURCE_DIR}/tests/fixtures/themes/${THEME}")
            set_tests_properties(repository-theme-${THEME} PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software" TIMEOUT 30)
        endforeach()
        add_test(NAME repository-journey-small COMMAND omastore --storefront-test --window-size 800x600)
        add_test(NAME repository-journey-hidpi COMMAND omastore --storefront-test --window-size 800x600)
        set_tests_properties(repository-journey-hidpi PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QT_SCALE_FACTOR=2")
        set_tests_properties(repository-journey repository-journey-small PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
    endif()
    if(OMASTORE_DEVELOPMENT_DATA)
        add_test(NAME native-lifecycle COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tests/lifecycle_process.py" "${CORE_BINARY}")
        set_tests_properties(native-lifecycle PROPERTIES TIMEOUT 20)
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
            "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/omastore-service" "${CORE_BINARY}" "${CMAKE_SOURCE_DIR}/data/repository/catalogue.json")
    endif()
    set_tests_properties(catalogue-http PROPERTIES TIMEOUT 30 SKIP_RETURN_CODE 77)
    set_tests_properties(core-process-contract native-startup PROPERTIES TIMEOUT 20)
    set_tests_properties(native-startup PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
endif()
