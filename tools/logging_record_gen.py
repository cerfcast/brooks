import csv
import dataclasses
from typing import List


@dataclasses.dataclass
class RecordField:
    name: str
    minimal: bool
    standard: bool
    extended: bool
    raw_ty: str
    ty: str
    descr: str

    def raw(self) -> str:
        field_name = self.name.replace("-", "_")
        ty = self.raw_ty
        if not self.minimal:
            ty = f"Option<{ty}>"
        result = f"""
    // {self.descr}"""
        if not self.minimal:
            result = result + f"""
    #[serde(skip_serializing_if = "Option::is_none")]"""
        result = result + f"""
    #[serde(rename = \"{self.name}\")]
    pub {field_name}: {ty},"""
        return result

    def typed(self) -> str:
        field_name = self.name.replace("-", "_")
        ty = self.ty
        if not self.minimal:
            ty = f"Option<{ty}>"
        result = f"""
    // {self.descr}
    pub {field_name}: {ty},"""
        return result


if __name__ == "__main__":
    fields: List[RecordField] = []

    with open("logging/fields.csv", newline='') as fh:
        reader = csv.reader(fh, delimiter=',', quotechar='|')
        for line in reader:
            line = list(map(lambda x: str.strip(x), line))
            name = line[0]
            raw_ty = line[1]
            ty = line[2]
            descr = line[3].replace("|", "")
            levels = line[4:]
            minimal = "Minimal" in levels
            extended = "Extended" in levels
            standard = "Standard" in levels
            fields.append(RecordField(name, minimal, standard, extended, raw_ty, ty, descr))

    with open("logging/template.rs") as fh:
        template = "".join(fh.readlines())
    
    with open("../src/cdni/logging/record.rs", "wt") as fh:
        fh.write(template)
        fh.write("#[derive(Debug, Clone, Deserialize, Serialize, Default)]\n")
        fh.write("pub struct LogRecordRaw {")
        for field in fields:
            fh.write(field.raw())
        fh.write("\n}\n")
        fh.write("#[derive(Debug, Clone, Default)]\n")
        fh.write("pub struct LogRecord {")
        for field in fields:
            fh.write(field.typed())
        fh.write("\n}\n")


"""
TODO
cs-hdr-Referer,Extended
cs-hdr-Range,Extended
cs-hdr-User-Agent,Standard,Extended
sc-hdr-Content-Type,Extended
sc-header-size-bytes,Extended
"""
