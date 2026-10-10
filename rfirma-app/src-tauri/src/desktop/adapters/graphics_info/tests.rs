use super::*;

fn adapter(description: &str, hardware_id: Option<&str>, provider: Option<&str>) -> DisplayAdapter {
    DisplayAdapter {
        description: description.to_owned(),
        driver_version: Some("32.0.15.6094".to_owned()),
        hardware_id: hardware_id.map(str::to_owned),
        provider: provider.map(str::to_owned),
    }
}

#[test]
fn a_windows_pci_adapter_takes_its_vendor_from_the_pci_id() {
    assert_eq!(
        gpu_of_adapter(adapter(
            "NVIDIA GeForce RTX 3060",
            Some(r"pci\ven_10de&dev_2504&subsys_397d1462"),
            Some("NVIDIA"),
        )),
        Gpu {
            vendor: "NVIDIA".to_owned(),
            driver: "NVIDIA GeForce RTX 3060".to_owned(),
            driver_version: Some("32.0.15.6094".to_owned()),
        }
    );
}

#[test]
fn the_pci_vendor_is_read_whatever_its_case() {
    assert_eq!(
        gpu_of_adapter(adapter(
            "Intel(R) UHD Graphics 620",
            Some(r"PCI\VEN_8086&DEV_5917"),
            None
        ))
        .vendor,
        "Intel"
    );
}

#[test]
fn a_windows_adapter_without_pci_id_takes_the_driver_provider() {
    assert_eq!(
        gpu_of_adapter(adapter(
            "Microsoft Basic Display Adapter",
            Some("root\\basicdisplay"),
            Some("Microsoft"),
        ))
        .vendor,
        "Microsoft"
    );
}

#[test]
fn system_profiler_gives_one_gpu_per_display_with_its_model_and_no_driver_version() {
    let json = r#"{"SPDisplaysDataType": [
        {"_name": "Apple M1", "sppci_model": "Apple M1", "sppci_vendor": "sppci_vendor_Apple"},
        {"_name": "Radeon", "sppci_model": "AMD Radeon Pro 5500M", "spdisplays_vendor": "sppci_vendor_amd"},
        {"_name": "GeForce", "sppci_model": "NVIDIA GeForce GT 750M", "spdisplays_vendor": "NVIDIA (0x10de)"}
    ]}"#;

    assert_eq!(
        gpus_from_system_profiler(json),
        [
            Gpu {
                vendor: "Apple".to_owned(),
                driver: "Apple M1".to_owned(),
                driver_version: None,
            },
            Gpu {
                vendor: "AMD".to_owned(),
                driver: "AMD Radeon Pro 5500M".to_owned(),
                driver_version: None,
            },
            Gpu {
                vendor: "NVIDIA".to_owned(),
                driver: "NVIDIA GeForce GT 750M".to_owned(),
                driver_version: None,
            },
        ]
    );
}

#[test]
fn system_profiler_output_that_is_not_json_gives_no_gpu() {
    assert_eq!(gpus_from_system_profiler("not json"), []);
}
