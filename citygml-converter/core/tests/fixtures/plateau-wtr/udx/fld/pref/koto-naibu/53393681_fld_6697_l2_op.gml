<?xml version='1.0' encoding='UTF-8'?>
<core:CityModel xmlns:brid="http://www.opengis.net/citygml/bridge/2.0" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:frn="http://www.opengis.net/citygml/cityfurniture/2.0" xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0" xmlns:sch="http://www.ascc.net/xml/schematron" xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:tun="http://www.opengis.net/citygml/tunnel/2.0" xmlns:tex="http://www.opengis.net/citygml/texturedsurface/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:app="http://www.opengis.net/citygml/appearance/2.0" xmlns:gen="http://www.opengis.net/citygml/generics/2.0" xmlns:dem="http://www.opengis.net/citygml/relief/2.0" xmlns:luse="http://www.opengis.net/citygml/landuse/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.2" xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:smil20="http://www.w3.org/2001/SMIL20/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:smil20lang="http://www.w3.org/2001/SMIL20/Language" xmlns:pbase="http://www.opengis.net/citygml/profiles/base/2.0" xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0" xsi:schemaLocation="https://www.geospatial.jp/iur/uro/3.2 ../../../../schemas/iur/uro/3.2/urbanObject.xsd http://www.opengis.net/citygml/2.0 http://schemas.opengis.net/citygml/2.0/cityGMLBase.xsd http://www.opengis.net/citygml/landuse/2.0 http://schemas.opengis.net/citygml/landuse/2.0/landUse.xsd http://www.opengis.net/citygml/building/2.0 http://schemas.opengis.net/citygml/building/2.0/building.xsd http://www.opengis.net/citygml/transportation/2.0 http://schemas.opengis.net/citygml/transportation/2.0/transportation.xsd http://www.opengis.net/citygml/generics/2.0 http://schemas.opengis.net/citygml/generics/2.0/generics.xsd http://www.opengis.net/citygml/relief/2.0 http://schemas.opengis.net/citygml/relief/2.0/relief.xsd http://www.opengis.net/citygml/cityobjectgroup/2.0 http://schemas.opengis.net/citygml/cityobjectgroup/2.0/cityObjectGroup.xsd http://www.opengis.net/gml http://schemas.opengis.net/gml/3.1.1/base/gml.xsd http://www.opengis.net/citygml/appearance/2.0 http://schemas.opengis.net/citygml/appearance/2.0/appearance.xsd">
	<gml:boundedBy>
		<gml:Envelope srsName="http://www.opengis.net/def/crs/EPSG/0/6697" srsDimension="3">
			<gml:lowerCorner>35.649999999998414 139.770625 1.96</gml:lowerCorner>
			<gml:upperCorner>35.65274999999843 139.775 4.69</gml:upperCorner>
		</gml:Envelope>
	</gml:boundedBy>
	<core:cityObjectMember>
		<wtr:WaterBody gml:id="fld_4b389494-8ef1-4a15-a2ad-7294520b8bad">
			<gml:name>江東内部河川流域浸水予想区域図（想定最大規模）</gml:name>
			<core:creationDate>2024-03-22</core:creationDate>
			<wtr:class codeSpace="../../../../codelists/WaterBody_class.xml">1140</wtr:class>
			<wtr:function codeSpace="../../../../codelists/WaterBody_function.xml">1</wtr:function>
			<wtr:lod1MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:Polygon>
							<gml:exterior>
								<gml:LinearRing>
									<gml:posList>35.652666669998425 139.774875 4.09 35.65268749999843 139.77490625 4.09 35.652708329998426 139.774875 4.09 35.652666669998425 139.774875 4.09</gml:posList>
								</gml:LinearRing>
							</gml:exterior>
						</gml:Polygon>
					</gml:surfaceMember>
					<gml:surfaceMember>
						<gml:Polygon>
							<gml:exterior>
								<gml:LinearRing>
									<gml:posList>35.652687499998436 139.77496875 4.09 35.652666669998425 139.7749375 3.49 35.65266666999844 139.77496875 3.49 35.652687499998436 139.77496875 4.09</gml:posList>
								</gml:LinearRing>
							</gml:exterior>
						</gml:Polygon>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</wtr:lod1MultiSurface>
			<uro:floodingRiskAttribute>
				<uro:RiverFloodingRiskAttribute>
					<uro:description codeSpace="../../../../codelists/RiverFloodingRiskAttribute_description.xml">4</uro:description>
					<uro:rank codeSpace="../../../../codelists/RiverFloodingRiskAttribute_rank.xml">1</uro:rank>
					<uro:adminType codeSpace="../../../../codelists/RiverFloodingRiskAttribute_adminType.xml">2</uro:adminType>
					<uro:scale codeSpace="../../../../codelists/RiverFloodingRiskAttribute_scale.xml">2</uro:scale>
				</uro:RiverFloodingRiskAttribute>
			</uro:floodingRiskAttribute>
			<uro:wtrDataQualityAttribute>
				<uro:DataQualityAttribute>
					<uro:geometrySrcDescLod1 codeSpace="../../../../codelists/DataQualityAttribute_geometrySrcDesc.xml">400</uro:geometrySrcDescLod1>
					<uro:thematicSrcDesc codeSpace="../../../../codelists/DataQualityAttribute_thematicSrcDesc.xml">400</uro:thematicSrcDesc>
				</uro:DataQualityAttribute>
			</uro:wtrDataQualityAttribute>
		</wtr:WaterBody>
	</core:cityObjectMember>
</core:CityModel>