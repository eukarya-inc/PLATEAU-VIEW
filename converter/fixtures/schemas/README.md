# Vendored i-UR 4.0 schemas

The XML Schema documents the converter writes into a converted package's
`schemas/` directory, so the output is self-contained the way a PLATEAU package
is: i-UR resolved through a relative path inside the package, CityGML referenced
remotely at `schemas.opengis.net`.

They are copied verbatim from `https://www.geospatial.jp/iur/schemas/<module>/4.0/`
and compiled into the binary, so conversion never touches the network — that
would make output depend on a server, and the published files are updated in
place.

| Module | File | Namespace |
| --- | --- | --- |
| uro | `iur/uro/4.0/urbanObject.xsd` | `https://www.geospatial.jp/iur/uro/4.0` |
| urc | `iur/urc/4.0/urbanCore.xsd` | `https://www.geospatial.jp/iur/urc/4.0` |
| urf | `iur/urf/4.0/urbanFunction.xsd` | `https://www.geospatial.jp/iur/urf/4.0` |
| urg | `iur/urg/4.0/statisticalGrid.xsd` | `https://www.geospatial.jp/iur/urg/4.0` |
| urt | `iur/urt/4.0/publicTransit.xsd` | `https://www.geospatial.jp/iur/urt/4.0` |

All five declare `version="4.0.0"`. Replacing them means re-downloading all five
together: they import one another by exact namespace, so a partial update pairs
versions that were never meant to meet.
