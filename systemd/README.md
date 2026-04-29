# Configure Systemd
* Check paths in `start.sh` and `mymodbus.service`
* Copy `mymodbus.service` to `/lib/systemd/system/`
* Run `sudo systemctl enable mymodbus.service`
* Run `sudo systemctl start mymodbus.service`
* Check status by running `sudo systemctl status mymodbus.service`

Output should be something like:
```
● mymodbus.service - Modbus reader for Raspberry Pi Zero 2 W
     Loaded: loaded (/lib/systemd/system/mymodbus.service; enabled; preset: enabled)
     Active: active (running) since Fri 2025-07-25 12:09:48 CEST; 47s ago
   Main PID: 510 (bash)
      Tasks: 3 (limit: 173)
        CPU: 41ms
     CGroup: /system.slice/mymodbus.service
             ├─510 /bin/bash /home/petste/mymodbus/start.sh
             └─518 /home/petste/MyModbus/mymodbus --config=/home/petste/MyModbus/config/mymodbus.conf

Jul 25 12:09:48 zeroeast systemd[1]: Started mymodbus.service - Modbus reader for Raspberry Pi Zero 2 W.
```

If the application for some reason prints anything to stdout/stderr, such in case of a panic,
the log for that can be found by using `journalctl -u mymodbus.service`.