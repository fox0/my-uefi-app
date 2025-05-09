run: esp/efi/boot/bootx64.efi OVMF_CODE.fd OVMF_VARS.fd
	qemu-system-x86_64 -enable-kvm \
		-drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
		-drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
		-drive format=raw,file=fat:rw:esp

esp/efi/boot/bootx64.efi: target/x86_64-unknown-uefi/debug/my-uefi-app.efi
	cp $< $@

OVMF_CODE.fd: /usr/share/OVMF/OVMF_CODE.fd
	cp $< $@

OVMF_VARS.fd: /usr/share/OVMF/OVMF_VARS.fd
	cp $< $@

target/x86_64-unknown-uefi/debug/my-uefi-app.efi: src/main.rs
	cargo build

clean:
	cargo clean
	rm -rf esp/efi/boot/bootx64.efi OVMF_CODE.fd OVMF_VARS.fd
