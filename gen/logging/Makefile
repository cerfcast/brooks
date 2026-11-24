validator: cdni.yang
	yang2dsdl -t config cdni.yang
validate:
	yang2dsdl -s -b cdni -t config -v fields.xml
clean:
	rm -rf cdni-config.dsrl cdni-config.rng cdni-config.sch cdni-gdefs-config.rng
