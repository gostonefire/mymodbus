# Configure Systemd
* Check paths in `start.sh` and `mymodbus.service`
* Copy `mymodbus.service` to `/lib/systemd/system/`
* Run `sudo systemctl enable mymodbus.service`
* Run `sudo systemctl start mymodbus.service`
* Check status by running `sudo systemctl status mymodbus.service`

Output should be something like:
```
● mymodbus.service - Modbus reader for Raspberry Pi Zero 2 W
     Loaded: loaded (/usr/lib/systemd/system/mymodbus.service; enabled; preset: enabled)
     Active: active (running) since Wed 2026-04-29 18:51:45 CEST; 7s ago
 Invocation: 2738993e280847cbbef6a2e2c3ec02ac
   Main PID: 48518 (bash)
      Tasks: 6 (limit: 176)
        CPU: 66ms
     CGroup: /system.slice/mymodbus.service
             ├─48518 /bin/bash /home/petste/MyModbus/start.sh
             └─48519 /home/petste/MyModbus/mymodbus --config=/home/petste/MyModbus/config/mymodbus.conf

Apr 29 18:51:45 zeroshed systemd[1]: Started mymodbus.service - Modbus reader for Raspberry Pi Zero 2 W.
```

If the application for some reason prints anything to stdout/stderr, such in case of a panic,
the log for that can be found by using `journalctl -u mymodbus.service`.