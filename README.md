# RPiTextShowSSD1306
Raspberry-Pi OLED-SSD1306 text server

----

`hello world` set text "hello world"

`@1:2+hello world` set page 1 line 2 = "hello world"

`@1:2~`  delete page 1 line 2

`@1:2?` query page 1 line 2

`@1+128,32:<base64>` set page 1 with width=128,height=32, decode data from base64

`@1~` delete page 1