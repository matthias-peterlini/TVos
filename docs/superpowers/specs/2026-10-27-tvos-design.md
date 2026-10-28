# TVos — Design

## Intento
Un "sistema operativo da TV" per Raspberry Pi 3B (1 GB RAM, Cortex-A53) collegato via HDMI. Si accende direttamente sulla UI, si comanda col telecomando della TV (HDMI-CEC) o, in alternativa, dal telefono via web. Deve essere molto leggero e fluido anche su hardware debole. Stile grafico: login.ticly.it (vetro scuro sfocato, sfondo montagne al tramonto, accenti ambra, font geometrico, schede arrotondate).

## Decisione architetturale (scelta mia, delegata dall'utente)
**Linux minimale (Buildroot, kernel mainline, aarch64) + UI e shell scritte in Rust.** Un kernel Rust puro bare-metal è scartato: non può avere Wi-Fi/Bluetooth (firmware Broadcom + driver), né browser/decodifica video, quindi YouTube e Prime sarebbero impossibili.

- Boot: kernel → init minimale (BusyBox) → `tvos-shell` a schermo intero. Nessun desktop, nessun display server: rendering software diretto su DRM/KMS dumb buffer (fallback `/dev/fb0`). Obiettivo: boot < 10 s, RAM a riposo < 150 MB.
- Input: HDMI-CEC (`/dev/cec0`, cec-ctl/libcec), tastiera, e telecomando web.
- Rete: Wi-Fi con `wpa_supplicant` + `iwd`-style UI nativa (scan, password con tastiera a schermo), Bluetooth con BlueZ (accoppiamento tastiere/casse).
- Telecomando web (fallback automatico): server HTTP in Rust sulla porta 8080, pagina mobile con D-pad, tastiera e touchpad; QR code a schermo.
- App incluse: Launcher, Impostazioni (Wi-Fi, Bluetooth, schermo, info), Monitor di sistema, Media Player locale, YouTube, Prime Video, Mirroring.
  - **YouTube**: browser leggero a schermo intero (Cog/WPE WebKit) su `youtube.com/tv`, controllato dal telecomando. Il suo peso è il rischio principale su 1 GB (swap su zram).
  - **Prime Video**: stesso browser. **Richiede DRM Widevine, non disponibile sul Pi 3B**: è best-effort e potrebbe non riprodurre. Alternativa: mostrarlo via mirroring.
  - **Mirroring telefono**: ricevitore AirPlay (UxPlay) e Google Cast-like se fattibile.
  - **Samsung DeX Wireless**: usa Miracast; il Wi-Fi del Pi 3B (BCM43438) ha supporto P2P limitato. Si tenta, ma senza garanzie: da verificare su hardware reale, non in VM.

## Goal
UI fluida e fedele allo screenshot; navigazione completa col telecomando; fallback web; Wi-Fi/Bluetooth utilizzabili; immagine SD avviabile; prova in VM.

## Non-goal
Multi-utente, app store, DRM Widevine, gaming, accelerazione GPU (VideoCore IV senza driver Rust).

## Anti-goal
Una UI che sembra bella solo nel simulatore ma non parte sull'immagine reale. Dati finti spacciati per funzionanti: le app o funzionano o dicono chiaramente che non sono disponibili.

## Stato di partenza (lavoro di Gemini)
Riutilizzabili: `tvos-core` (Surface, Color, font), `tvos-cec`, `tvos-remote-web`. Da rifare: UI (non corrisponde allo screenshot), `font8x16.bin` vuoto in `tvos-core`, app YouTube/Prime con dati finti. Da eliminare: kernel `tvos-baremetal` e `tvos-uefi` (non servono più; il UEFI resta solo finché la VM non gira con Linux).

## Struttura target
```
crates/tvos-core        rendering software, tema Ticly, font TTF (fontdue), blur
crates/tvos-shell       binario unico: launcher + app native, loop input
crates/tvos-cec         input CEC
crates/tvos-remote-web  telecomando web + QR
crates/tvos-net         Wi-Fi/Bluetooth (wpa_supplicant/BlueZ via D-Bus/socket)
crates/tvos-simulator   finestra desktop per sviluppo rapido
os/buildroot            defconfig Pi 3B + defconfig x86_64 per la VM, overlay
```

## VM
Per la VM serve un Linux x86_64 con la stessa UI. Su questo PC (Windows, niente WSL né Docker) servono strumenti Linux per compilare Buildroot: **WSL2** (download ~ centinaia di MB, possibile riavvio). VirtualBox 7.2 è già installato e basta per x86_64; QEMU è necessario solo per emulare davvero il Pi (lento, opzionale).

## Lasciato al builder
Scelta delle librerie (font, immagini), dettagli del layout, ordine di sviluppo, tuning di memoria e boot.
