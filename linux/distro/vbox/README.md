# VirtualBox

1. Convert qcow2 → VDI: `qemu-img convert -O vdi ../qemu/jarvis-linux.qcow2 jarvis-linux.vdi`
2. Create VM: 2 CPU, 2048 MB, attach VDI, NIC virtio-net or Intel PRO/1000.
3. Optional: `VBoxManage export` to `jarvis-linux.ova`.

Guest additions are not required on day 1. Set `JARVIS_BOOT=vbox` in the guest environment.
