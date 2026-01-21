/*
        for i in 1..5 {
            let hosts_ids = cpal::available_hosts();

            for host_id in hosts_ids {
                println!("==== HOST {:?}", host_id);

                let host_result = cpal::host_from_id(host_id);

                if let Ok(host) = host_result {
                    let devices_result = host.devices();
                    if let Ok(devices) = devices_result {

                        for device in devices {
                            let devicename_result = device.name();
                            if let Ok(devicename) = devicename_result {
                                println!("   ----{:?}", devicename);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        return Ok(());
    */