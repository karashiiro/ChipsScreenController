# (WIP) ChipsScreenController

A thing to control the USB diagnostics screen I bought off Temu ([product link](https://www.temu.com/goods.html?goods_id=601099577316872))
without using the [official app](https://www.adrive.com/public/nRJGGr/USBPCMonitorENG_3_0_3.zip). The screen's only identifier that I could
find was the string `USB35INCHIPSV2` in the binary. When I found that, I mistakenly read it as "USB 3.5in Chips V2" instead of
"USB 3.5 Inch IPS V2", so I'm just calling it Chips for short.

The official app is a bit slower than I was hoping for, so I made this instead to optimize it on my own.
