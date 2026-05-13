```
VITE_API_HOST=radxa0:8080 bun run dev --host
```

`CC_arm_unknown_linux_musleabihf=arm-linux-musleabihf-gcc CARGO_TARGET_ARM_UNKNOWN_LINUX_MUSLEABIHF_LINKER=arm-linux-musleabihf-gcc cargo build --target arm-unknown-linux-musleabihf --release`

`sudo usermod -aG dialout $USER`

`newgrp dialout`

```
sudo systemctl set-default multi-user.target
sudo systemctl set-default graphical.target
```

https://www.lddgo.net/en/coordinate/nmea-0183-parser

```
sudo vim /etc/systemd/system/rover.service
```

```
[Unit]
Description=UM980 Rover
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
Restart=always
WorkingDirectory=/home/radxa
EnvironmentFile=/home/radxa/rover.env
ExecStart=/home/radxa/rover
User=root
Group=dialout

[Install]
WantedBy=multi-user.target
```

```
sudo systemctl daemon-reload
sudo systemctl enable rover.service
```

```
rustup target add aarch64-unknown-linux-musl
```

```
cargo build --target aarch64-unknown-linux-musl --release
```

```
sudo usermod -a -G spidev $USER
```

```
nmcli connection show
nmcli connection show --active
sudo nmcli device wifi list
sudo nmcli connection add type wifi con-name "For Rover" ssid "For Rover" wifi-sec.key-mgmt wpa-psk wifi-sec.psk "XXXXXXXXX"
sudo nmcli con up "For Rover"
```

```
sudo journalctl -u rover.service -f
```

```
sudo systemctl restart rover.service
sudo systemctl stop rover.service
sudo systemctl start rover.service
```

```
ST7789 Pin,Radxa Physical Pin,Radxa Pin Name,Notes
VCC,Pin 1,3.3V Power,ST7789 logic runs on 3.3V.
GND,Pin 20,Ground,Any GND pin works.
SCL / CLK,Pin 23,SPI3_CLK_M1,The hardware clock signal.
SDA / MOSI,Pin 19,SPI3_MOSI_M1,The hardware data line.
CS (Optional),Pin 24,SPI3_CS0_M1,"Chip Select. (If your screen lacks a CS pin, it is hardwired internally)."
DC,Pin 22,GPIO3_C1,Data/Command toggle. You will control this via Rust.
RST / RES,Pin 18,GPIO3_B2,Reset toggle. You will control this via Rust.
BLK (Optional),Pin 17,3.3V Power,Backlight. Tie to 3.3V to keep the screen permanently on.
```

```
sudo dtc -@ -I dts -O dtb -o /boot/dtbo/st7789.dtbo st7789-overlay.dts
sudo dmesg | grep -E "spi|dbi"
sudo modprobe panel_mipi_dbi
```

```
 printf '\x4d\x49\x50\x49\x20\x44\x42\x49\x00\x00\x00\x00\x00\x00\x00\x01\x36\x01\x08\x3A\x01\x05\x35\x01\x00\xB2\x05\x0C\x0C\x00\x33\x33\xB7\x01\x35\xBB\x01\x19\xC0\x01\x2C\xC2\x01\x01\xC3\x01\x12\xC4\x01\x20\xC6\x01\x0F\xD0\x02\xA4\xA1\xE0\x0E\xD0\x04\x0D\x11\x13\x2B\x3F\x54\x4C\x18\x0D\x0B\x1F\x23\xE1\x0E\xD0\x04\x0C\x11\x13\x2C\x3F\x44\x51\x2F\x1F\x1F\x20\x23\x21\x00\x11\x00\x00\x01\x78\x29\x00' > st7789v-waveshare.bin
sudo cp st7789v-waveshare.bin /lib/firmware/
hexdump -C /lib/firmware/st7789v-waveshare.bin | head
```

```
ls /proc/device-tree/ | grep spi
ls /proc/device-tree/spi@fe640000/display@0/
```

```
radxa@radxa0:~$ find /lib/modules/$(uname -r) -name "*dbi*"
/lib/modules/6.1.84-10-rk2410-nocsf/kernel/drivers/gpu/drm/drm_mipi_dbi.ko
/lib/modules/6.1.84-10-rk2410-nocsf/kernel/drivers/gpu/drm/panel-mipi-dbi.ko
```

```
ls -d /sys/bus/platform/devices/*.spi
```

```
sudo cat /sys/kernel/debug/pinctrl/pinctrl-rockchip-pinctrl/pinmux-pins | grep spi3
```

```
sudo rm /etc/console-setup/cached_*
sudo setupcon --save
```

sudo cat /etc/default/console-setup

```
ACTIVE_CONSOLES="/dev/tty[1-6]"
CHARMAP="UTF-8"
CODESET="Lat38"
FONTFACE="Terminus"
FONTSIZE="12x6"
VIDEOMODE=
```

```
sudo fbi -d /dev/fb0 -T 1 -a progressive_1920x1080_319kb.jpg
```

```
startx -- -depth 24
```

```
DISPLAY=:0 xrandr
```

```
radxa@radxa0:~$ cat /sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq
1800000
radxa@radxa0:~$ cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq
1104000
radxa@radxa0:~$ cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq
1416000
radxa@radxa0:~$ cat /sys/class/thermal/thermal_zone0/temp
80555
radxa@radxa0:~$ for z in /sys/class/thermal/thermal_zone*; do
  echo "$z: $(cat $z/type) -> $(cat $z/temp)"
done
/sys/class/thermal/thermal_zone0: soc-thermal -> 78750
/sys/class/thermal/thermal_zone1: gpu-thermal -> 74444
radxa@radxa0:~$ cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
ondemand
```

```
https://github.com/MarcA711/Rockchip-FFmpeg-Builds/releases
```

```
mpv --vo=drm --hwdec=rkmpp --osd-level=3 BigBuckBunny_640x360.m4v
```

```
echo spi3.0 | sudo tee /sys/bus/spi/drivers/panel-mipi-dbi-spi/unbind
echo spi3.0 | sudo tee /sys/bus/spi/drivers/panel-mipi-dbi-spi/bind
```

```
ffmpeg -decoders | grep rkmpp
```

```
startx -depth 24
```

```
cat /sys/class/graphics/fb0/bits_per_pixel
cat /sys/class/graphics/fb0/virtual_size
cat /sys/class/graphics/fb0/stride
```

```
sudo NTRIP_EMAIL=<email>  ./nmea-tcp
```

```
sudo usbreset 1a86:7523
```

```
rtk2go.com:2101
IndiaTN02
```

### For building package with bluetooth feature:

Currently building for `musl` is not working

```
 CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo build --target aarch64-unknown-linux-gnu --release
```
