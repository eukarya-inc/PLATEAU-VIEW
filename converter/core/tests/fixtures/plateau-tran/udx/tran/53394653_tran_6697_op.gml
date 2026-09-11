<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0" xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:pbase="http://www.opengis.net/citygml/profiles/base/2.0" xmlns:smil20lang="http://www.w3.org/2001/SMIL20/Language" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:smil20="http://www.w3.org/2001/SMIL20/" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1" xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0" xmlns:luse="http://www.opengis.net/citygml/landuse/2.0" xmlns:gen="http://www.opengis.net/citygml/generics/2.0" xmlns:dem="http://www.opengis.net/citygml/relief/2.0" xmlns:app="http://www.opengis.net/citygml/appearance/2.0" xmlns:tex="http://www.opengis.net/citygml/texturedsurface/2.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:tun="http://www.opengis.net/citygml/tunnel/2.0" xmlns:sch="http://www.ascc.net/xml/schematron" xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0" xmlns:frn="http://www.opengis.net/citygml/cityfurniture/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0" xmlns:brid="http://www.opengis.net/citygml/bridge/2.0" xsi:schemaLocation="https://www.geospatial.jp/iur/uro/3.1 ../../schemas/iur/uro/3.1/urbanObject.xsd http://www.opengis.net/citygml/2.0 http://schemas.opengis.net/citygml/2.0/cityGMLBase.xsd http://www.opengis.net/citygml/landuse/2.0 http://schemas.opengis.net/citygml/landuse/2.0/landUse.xsd http://www.opengis.net/citygml/building/2.0 http://schemas.opengis.net/citygml/building/2.0/building.xsd http://www.opengis.net/citygml/transportation/2.0 http://schemas.opengis.net/citygml/transportation/2.0/transportation.xsd http://www.opengis.net/citygml/generics/2.0 http://schemas.opengis.net/citygml/generics/2.0/generics.xsd http://www.opengis.net/citygml/cityobjectgroup/2.0 http://schemas.opengis.net/citygml/cityobjectgroup/2.0/cityObjectGroup.xsd http://www.opengis.net/gml http://schemas.opengis.net/gml/3.1.1/base/gml.xsd http://www.opengis.net/citygml/cityfurniture/2.0 http://schemas.opengis.net/citygml/cityfurniture/2.0/cityFurniture.xsd http://www.opengis.net/citygml/vegetation/2.0 http://schemas.opengis.net/citygml/vegetation/2.0/vegetation.xsd http://www.opengis.net/citygml/appearance/2.0 http://schemas.opengis.net/citygml/appearance/2.0/appearance.xsd">
	<gml:boundedBy>
		<gml:Envelope srsName="http://www.opengis.net/def/crs/EPSG/0/6697" srsDimension="3">
			<gml:lowerCorner>35.70804956684993 139.78715470177093 0</gml:lowerCorner>
			<gml:upperCorner>35.71697615109303 139.80021956325191 2.7107658593039097</gml:upperCorner>
		</gml:Envelope>
	</gml:boundedBy>
<core:cityObjectMember xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0" xmlns:pbase="http://www.opengis.net/citygml/profiles/base/2.0" xmlns:smil20lang="http://www.w3.org/2001/SMIL20/Language" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:smil20="http://www.w3.org/2001/SMIL20/" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1" xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0" xmlns:luse="http://www.opengis.net/citygml/landuse/2.0" xmlns:gen="http://www.opengis.net/citygml/generics/2.0" xmlns:dem="http://www.opengis.net/citygml/relief/2.0" xmlns:app="http://www.opengis.net/citygml/appearance/2.0" xmlns:tex="http://www.opengis.net/citygml/texturedsurface/2.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:tun="http://www.opengis.net/citygml/tunnel/2.0" xmlns:sch="http://www.ascc.net/xml/schematron" xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0" xmlns:frn="http://www.opengis.net/citygml/cityfurniture/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0" xmlns:brid="http://www.opengis.net/citygml/bridge/2.0">
		<tran:Road gml:id="tran_4e9e51cd-e581-41b6-ba93-2089ce31c6fd">
			<core:creationDate>2024-03-15</core:creationDate>
			<tran:class codeSpace="../../codelists/TransportationComplex_class.xml">1040</tran:class>
			<tran:function codeSpace="../../codelists/Road_function.xml">9020</tran:function>
			<tran:usage codeSpace="../../codelists/Road_usage.xml">9</tran:usage>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tra_04806b92-8819-4fad-b0fa-24239d9f3a07">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1000</tran:function>
					<tran:lod2MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_583482c1-37f2-4645-b320-5ff951a9a8f8">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.7087359009 139.78802329168158 0 35.70877049105351 139.7877254326408 0 35.70870532084925 139.78772193326913 0 35.70867082075736 139.78801957098705 0 35.7087359009 139.78802329168158 0</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod2MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:lod1MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:Polygon>
							<gml:exterior>
								<gml:LinearRing>
									<gml:posList>35.7087359009 139.78802329168158 0 35.70877049105351 139.7877254326408 0 35.70870532084925 139.78772193326913 0 35.70867082075736 139.78801957098705 0 35.7087359009 139.78802329168158 0</gml:posList>
								</gml:LinearRing>
							</gml:exterior>
						</gml:Polygon>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod1MultiSurface>
			<tran:lod2MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:CompositeSurface>
							<gml:surfaceMember xlink:href="#poly_583482c1-37f2-4645-b320-5ff951a9a8f8"/>
						</gml:CompositeSurface>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod2MultiSurface>
			<uro:tranDataQualityAttribute>
				<uro:DataQualityAttribute>
					<uro:geometrySrcDescLod1 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod1>
					<uro:geometrySrcDescLod2 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod2>
					<uro:geometrySrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">999</uro:geometrySrcDescLod3>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">023</uro:thematicSrcDesc>
					<uro:appearanceSrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_appearanceSrcDesc.xml">99</uro:appearanceSrcDescLod3>
					<uro:publicSurveyDataQualityAttribute>
						<uro:PublicSurveyDataQualityAttribute>
							<uro:srcScaleLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod1>
							<uro:srcScaleLod2 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod2>
							<uro:publicSurveySrcDescLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">023</uro:publicSurveySrcDescLod1>
							<uro:publicSurveySrcDescLod2 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">023</uro:publicSurveySrcDescLod2>
						</uro:PublicSurveyDataQualityAttribute>
					</uro:publicSurveyDataQualityAttribute>
				</uro:DataQualityAttribute>
			</uro:tranDataQualityAttribute>
			<uro:roadStructureAttribute>
				<uro:RoadStructureAttribute>
					<uro:sectionType codeSpace="../../codelists/RoadStructureAttribute_sectionType.xml">1</uro:sectionType>
				</uro:RoadStructureAttribute>
			</uro:roadStructureAttribute>
		</tran:Road>
	</core:cityObjectMember>
	
<core:cityObjectMember xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0" xmlns:pbase="http://www.opengis.net/citygml/profiles/base/2.0" xmlns:smil20lang="http://www.w3.org/2001/SMIL20/Language" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:smil20="http://www.w3.org/2001/SMIL20/" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1" xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0" xmlns:luse="http://www.opengis.net/citygml/landuse/2.0" xmlns:gen="http://www.opengis.net/citygml/generics/2.0" xmlns:dem="http://www.opengis.net/citygml/relief/2.0" xmlns:app="http://www.opengis.net/citygml/appearance/2.0" xmlns:tex="http://www.opengis.net/citygml/texturedsurface/2.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:tun="http://www.opengis.net/citygml/tunnel/2.0" xmlns:sch="http://www.ascc.net/xml/schematron" xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0" xmlns:frn="http://www.opengis.net/citygml/cityfurniture/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0" xmlns:brid="http://www.opengis.net/citygml/bridge/2.0">
		<tran:Road gml:id="tran_1fc77a63-17ae-4507-a25f-97e1dab09fb1">
			<core:creationDate>2024-03-15</core:creationDate>
			<tran:class codeSpace="../../codelists/TransportationComplex_class.xml">1040</tran:class>
			<tran:function codeSpace="../../codelists/Road_function.xml">3</tran:function>
			<tran:usage codeSpace="../../codelists/Road_usage.xml">3</tran:usage>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tra_1fe05bd0-ae9b-4c07-8023-ffc28bee8863">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">2000</tran:function>
					<tran:lod2MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_3e0eb676-21cd-4c57-9948-f3e010ac677a">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.712442849289005 139.79206673543874 0 35.71245069661615 139.792001140701 0 35.712396880771514 139.79199144820166 0 35.71238903165213 139.79205705837 0 35.712442849289005 139.79206673543874 0</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod2MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tra_e13f0eda-519d-498e-a8fe-fbb7e2bec74d">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1020</tran:function>
					<tran:lod2MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_b141921d-ea0e-4866-bb1d-9ae83f29ea3f">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71240672652029 139.7923634521184 0 35.71240818256345 139.79235649410174 0 35.712429251422236 139.79218038970112 0 35.71237547783655 139.79217034781715 0 35.71235436764637 139.7923467958421 0 35.71240672652029 139.7923634521184 0</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod2MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tra_f5165d05-096c-425c-b2f7-ce0d57c22876">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1000</tran:function>
					<tran:lod2MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_89f70e22-7fc9-4887-8e4e-e43362635572">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71243056097056 139.7921694430651 0 35.712442849289005 139.79206673543874 0 35.71238903165213 139.79205705837 0 35.712376787384365 139.79215940229355 0 35.71243056097056 139.7921694430651 0</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod2MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_9e2b988e-7dcd-48d3-83cf-e9352fce99f8">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">2000</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_5e08bd38-2a12-4e8e-8c2d-d22ece31ba02">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71245326895803 139.7919935285566 1.8320493678812626 35.71239920534939 139.79198372918324 1.8729501767220686 35.71244512576481 139.79205908556935 1.8308423974087538 35.71245326895803 139.7919935285566 1.8320493678812626</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_fbe5e8db-f557-4a75-8113-1b382ac02e9c">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71244512576481 139.79205908556935 1.8308423974087538 35.71239920534939 139.79198372918324 1.8729501767220686 35.7123910621614 139.7920492861527 1.8717432062495312 35.71244512576481 139.79205908556935 1.8308423974087538</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_b4a61f1b-7823-450b-bb5d-9a7939b03e36">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1000</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_8949557f-4da4-4174-a3a0-9c0cc3841a20">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71244512576481 139.79205908556935 1.830842397408723 35.7123910621614 139.7920492861527 1.871743206249566 35.71237737036831 139.79215951164898 1.869713842222425 35.71244512576481 139.79205908556935 1.830842397408723</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_8a44ccc1-8cc5-4cd7-8c87-b9a2236fca5e">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71244512576481 139.79205908556935 1.830842397408723 35.71237737036831 139.79215951164898 1.869713842222425 35.71243139820443 139.7921695990103 1.8288077333687993 35.71244512576481 139.79205908556935 1.830842397408723</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_21f33820-1e95-403e-85dc-da7f3f2d081d">
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1020</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_8e3a2e81-cdad-4102-b0e0-c1e6ea3c665e">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71240818256345 139.79235649410174 1.8253668044319729 35.71243003948052 139.79218053734246 1.8286063475906071 35.71235445172107 139.79234401583932 1.8663169299822429 35.71240818256345 139.79235649410174 1.8253668044319729</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_885a88c0-1fc3-4ff8-b47b-c352fd509dde">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71235445172107 139.79234401583932 1.8663169299822429 35.71243003948052 139.79218053734246 1.8286063475906071 35.712376011935625 139.79217044763675 1.869512499474514 35.71235445172107 139.79234401583932 1.8663169299822429</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:auxiliaryTrafficArea>
				<tran:AuxiliaryTrafficArea gml:id="ata_3c175dd2-7a21-4395-b658-cbfe658c9032">
					<tran:function codeSpace="../../codelists/AuxiliaryTrafficArea_function.xml">3000</tran:function>
					<tran:lod2MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_0302618b-5ba8-4339-a467-c5e9e1cd466c">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.712429251422236 139.79218038970112 0 35.71243056097056 139.7921694430651 0 35.712376787384365 139.79215940229355 0 35.71237547783655 139.79217034781715 0 35.712429251422236 139.79218038970112 0</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod2MultiSurface>
				</tran:AuxiliaryTrafficArea>
			</tran:auxiliaryTrafficArea>
			<tran:auxiliaryTrafficArea>
				<tran:AuxiliaryTrafficArea gml:id="atr_eae7524f-56d2-48b3-b5bd-c74d3b9b5125">
					<tran:function codeSpace="../../codelists/AuxiliaryTrafficArea_function.xml">3000</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_a9edc244-3edd-4437-a121-b376db90dc27">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71243003948052 139.79218053734246 1.8286063475906176 35.71243139820443 139.7921695990103 1.8288077333687944 35.712376011935625 139.79217044763675 1.8695124994745635 35.71243003948052 139.79218053734246 1.8286063475906176</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_aff0ed38-fc0d-4f12-b451-6f77323ac580">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.712376011935625 139.79217044763675 1.8695124994745635 35.71243139820443 139.7921695990103 1.8288077333687944 35.71237737036831 139.79215951164898 1.8697138422224724 35.712376011935625 139.79217044763675 1.8695124994745635</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:AuxiliaryTrafficArea>
			</tran:auxiliaryTrafficArea>
			<tran:lod1MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:Polygon>
							<gml:exterior>
								<gml:LinearRing>
									<gml:posList>35.71240818256345 139.79235649410174 0 35.71245069661615 139.792001140701 0 35.712396880771514 139.79199144820166 0 35.71235436764637 139.7923467958421 0 35.71240672652029 139.7923634521184 0 35.71240818256345 139.79235649410174 0</gml:posList>
								</gml:LinearRing>
							</gml:exterior>
						</gml:Polygon>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod1MultiSurface>
			<tran:lod2MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:CompositeSurface>
							<gml:surfaceMember xlink:href="#poly_3e0eb676-21cd-4c57-9948-f3e010ac677a"/>
							<gml:surfaceMember xlink:href="#poly_b141921d-ea0e-4866-bb1d-9ae83f29ea3f"/>
							<gml:surfaceMember xlink:href="#poly_89f70e22-7fc9-4887-8e4e-e43362635572"/>
							<gml:surfaceMember xlink:href="#poly_0302618b-5ba8-4339-a467-c5e9e1cd466c"/>
						</gml:CompositeSurface>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod2MultiSurface>
			<tran:lod3MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:CompositeSurface>
							<gml:surfaceMember xlink:href="#poly_5e08bd38-2a12-4e8e-8c2d-d22ece31ba02"/>
							<gml:surfaceMember xlink:href="#poly_fbe5e8db-f557-4a75-8113-1b382ac02e9c"/>
							<gml:surfaceMember xlink:href="#poly_8949557f-4da4-4174-a3a0-9c0cc3841a20"/>
							<gml:surfaceMember xlink:href="#poly_8a44ccc1-8cc5-4cd7-8c87-b9a2236fca5e"/>
							<gml:surfaceMember xlink:href="#poly_8e3a2e81-cdad-4102-b0e0-c1e6ea3c665e"/>
							<gml:surfaceMember xlink:href="#poly_885a88c0-1fc3-4ff8-b47b-c352fd509dde"/>
							<gml:surfaceMember xlink:href="#poly_a9edc244-3edd-4437-a121-b376db90dc27"/>
							<gml:surfaceMember xlink:href="#poly_aff0ed38-fc0d-4f12-b451-6f77323ac580"/>
						</gml:CompositeSurface>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod3MultiSurface>
			<uro:tranDataQualityAttribute>
				<uro:DataQualityAttribute>
					<uro:geometrySrcDescLod1 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod1>
					<uro:geometrySrcDescLod2 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod2>
					<uro:geometrySrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod3>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">023</uro:thematicSrcDesc>
					<uro:appearanceSrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_appearanceSrcDesc.xml">5</uro:appearanceSrcDescLod3>
					<uro:lodType codeSpace="../../codelists/Road_lodType.xml">3.0</uro:lodType>
					<uro:publicSurveyDataQualityAttribute>
						<uro:PublicSurveyDataQualityAttribute>
							<uro:srcScaleLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod1>
							<uro:srcScaleLod2 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod2>
							<uro:srcScaleLod3 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">2</uro:srcScaleLod3>
							<uro:publicSurveySrcDescLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">023</uro:publicSurveySrcDescLod1>
							<uro:publicSurveySrcDescLod2 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">023</uro:publicSurveySrcDescLod2>
							<uro:publicSurveySrcDescLod3 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">011</uro:publicSurveySrcDescLod3>
						</uro:PublicSurveyDataQualityAttribute>
					</uro:publicSurveyDataQualityAttribute>
				</uro:DataQualityAttribute>
			</uro:tranDataQualityAttribute>
			<uro:roadStructureAttribute>
				<uro:RoadStructureAttribute>
					<uro:sectionType codeSpace="../../codelists/RoadStructureAttribute_sectionType.xml">4</uro:sectionType>
				</uro:RoadStructureAttribute>
			</uro:roadStructureAttribute>
		</tran:Road>
	</core:cityObjectMember>
	
<core:cityObjectMember xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0" xmlns:pbase="http://www.opengis.net/citygml/profiles/base/2.0" xmlns:smil20lang="http://www.w3.org/2001/SMIL20/Language" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:smil20="http://www.w3.org/2001/SMIL20/" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1" xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0" xmlns:luse="http://www.opengis.net/citygml/landuse/2.0" xmlns:gen="http://www.opengis.net/citygml/generics/2.0" xmlns:dem="http://www.opengis.net/citygml/relief/2.0" xmlns:app="http://www.opengis.net/citygml/appearance/2.0" xmlns:tex="http://www.opengis.net/citygml/texturedsurface/2.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:tun="http://www.opengis.net/citygml/tunnel/2.0" xmlns:sch="http://www.ascc.net/xml/schematron" xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0" xmlns:frn="http://www.opengis.net/citygml/cityfurniture/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0" xmlns:brid="http://www.opengis.net/citygml/bridge/2.0">
		<tran:Road gml:id="tran_0f0a7c5b-d7b6-4349-b928-da6c101df61a">
			<core:creationDate>2025-03-21</core:creationDate>
			<tran:class codeSpace="../../codelists/TransportationComplex_class.xml">1040</tran:class>
			<tran:function codeSpace="../../codelists/Road_function.xml">9020</tran:function>
			<tran:usage>Null</tran:usage>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_180b8009-3115-4095-ae35-f07715ebb6d1">
					<core:creationDate>2025-03-21</core:creationDate>
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1000</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_12ee9405-b94b-4412-b704-df94754805f6">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71692909072055 139.81643763110807 -0.04266710110876677 35.71694400942787 139.81646170255922 -0.004000000000000011 35.7169334684518 139.8164389195677 -0.036000433970230006 35.71692909072055 139.81643763110807 -0.04266710110876677</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_07e5ab18-53b9-4762-884a-2068e10a04d7">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71694400942787 139.81646170255922 -0.004000000000000011 35.71692909072055 139.81643763110807 -0.04266710110876677 35.71691957007114 139.81643408705636 -0.05543142700976097 35.71694400942787 139.81646170255922 -0.004000000000000011</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_e906f28c-6c46-458b-9184-cf62fccf0985">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.7169334684518 139.8164389195677 -0.036000433970230006 35.71694400942787 139.81646170255922 -0.004000000000000011 35.716938772042816 139.81644023186524 -0.02600043326242485 35.7169334684518 139.8164389195677 -0.036000433970230006</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_927a6159-ef80-441b-b2aa-e316bd9b23ca">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716938772042816 139.81644023186524 -0.02600043326242485 35.71694400942787 139.81646170255922 -0.004000000000000011 35.716942398341516 139.8164409522322 -0.02600043326242485 35.716938772042816 139.81644023186524 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_1a0ecb5b-b384-4232-9090-83d5f79aa0b7">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716942398341516 139.8164409522322 -0.02600043326242485 35.71694400942787 139.81646170255922 -0.004000000000000011 35.71694866811655 139.81645529892856 -0.034333767185595776 35.716942398341516 139.8164409522322 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_c64147e8-2d5d-42a8-9bac-9e78966b50e5">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716942398341516 139.8164409522322 -0.02600043326242485 35.71694866811655 139.81645529892856 -0.034333767185595776 35.716947187808294 139.8164416684421 -0.02600043326242485 35.716942398341516 139.8164409522322 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_7525271a-e9a6-4140-9cd9-152c26cdecba">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716947187808294 139.8164416684421 -0.02600043326242485 35.71694866811655 139.81645529892856 -0.034333767185595776 35.71695142677961 139.81644208187942 -0.02600043326242485 35.716947187808294 139.8164416684421 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_f786cd25-51c7-4b7a-b240-6f849fda995a">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695142677961 139.81644208187942 -0.02600043326242485 35.71694866811655 139.81645529892856 -0.034333767185595776 35.71695838989647 139.81644268871074 -0.026499702256160163 35.71695142677961 139.81644208187942 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_46364b7a-9ad6-4f6a-9d0e-32b2f400c8a6">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695838989647 139.81644268871074 -0.026499702256160163 35.71694866811655 139.81645529892856 -0.034333767185595776 35.71698893386227 139.8164584492967 -0.05188638116543241 35.71695838989647 139.81644268871074 -0.026499702256160163</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_7e97757a-97de-4dff-ab9f-518c36d364a7">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695838989647 139.81644268871074 -0.026499702256160163 35.71698893386227 139.8164584492967 -0.05188638116543241 35.716987709902 139.8164494078163 -0.10989382736640642 35.71695838989647 139.81644268871074 -0.026499702256160163</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_dd61294f-2e53-4428-a461-79b27f67c06b">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695838989647 139.81644268871074 -0.026499702256160163 35.716987709902 139.8164494078163 -0.10989382736640642 35.716987813803236 139.81644385439748 -0.06649970508736441 35.71695838989647 139.81644268871074 -0.026499702256160163</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_f0bf9674-e4e1-4e29-a8e5-c0e61577f3c2">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71691957007114 139.81643408705636 -0.05543142700976097 35.71690299398692 139.81642198739206 -0.08469191976380117 35.71694145955276 139.81646518215018 -0.0040263885589877195 35.71691957007114 139.81643408705636 -0.05543142700976097</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_7d53fa22-bd48-4762-94d0-60a54a239757">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71690299398692 139.81642198739206 -0.08469191976380117 35.71691957007114 139.81643408705636 -0.05543142700976097 35.71690569395532 139.81641840760707 -0.08463302587251621 35.71690299398692 139.81642198739206 -0.08469191976380117</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_06f1fd2b-cc3f-4b73-8933-a662b33b7c4b">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71691957007114 139.81643408705636 -0.05543142700976097 35.71694145955276 139.81646518215018 -0.0040263885589877195 35.71694400942787 139.81646170255922 -0.004000000000000011 35.71691957007114 139.81643408705636 -0.05543142700976097</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_d9607a32-6dde-4c0a-860a-c2d6cfed93d8">
					<core:creationDate>2025-03-21</core:creationDate>
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1010</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_27d3a19d-f179-45d6-9e65-792c7c1b5f21">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71696697185749 139.8163957115638 -0.12899574417247095 35.716987813803236 139.81644385439748 -0.06649970508738079 35.716988703079714 139.81639632288298 -0.12399574381856837 35.71696697185749 139.8163957115638 -0.12899574417247095</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_32d26510-6b43-4a2a-86e6-34727747b531">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716987813803236 139.81644385439748 -0.06649970508738079 35.71696697185749 139.8163957115638 -0.12899574417247095 35.71695838989647 139.81644268871074 -0.026499702256160163 35.716987813803236 139.81644385439748 -0.06649970508738079</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_15208d6a-3b70-4bde-943d-36602c0a49b9">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695838989647 139.81644268871074 -0.026499702256160163 35.71696697185749 139.8163957115638 -0.12899574417247095 35.71695142677961 139.81644208187942 -0.02600043326242485 35.71695838989647 139.81644268871074 -0.026499702256160163</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_019f9973-3b59-4aee-a64d-812bc671fa54">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71695142677961 139.81644208187942 -0.02600043326242485 35.71696697185749 139.8163957115638 -0.12899574417247095 35.716947187808294 139.8164416684421 -0.02600043326242485 35.71695142677961 139.81644208187942 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_91bb7be8-9e87-4a3b-a157-102fcc278bbe">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716947187808294 139.8164416684421 -0.02600043326242485 35.71696697185749 139.8163957115638 -0.12899574417247095 35.716942398341516 139.8164409522322 -0.02600043326242485 35.716947187808294 139.8164416684421 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_a135c773-aa7b-4e4c-9e2b-7a46835168da">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716942398341516 139.8164409522322 -0.02600043326242485 35.71696697185749 139.8163957115638 -0.12899574417247095 35.716924685818086 139.81639414199563 -0.13786133488567665 35.716942398341516 139.8164409522322 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_8a54b23f-d742-4a8a-aab2-dbcdc28e5d01">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716942398341516 139.8164409522322 -0.02600043326242485 35.716924685818086 139.81639414199563 -0.13786133488567665 35.716938772042816 139.81644023186524 -0.02600043326242485 35.716942398341516 139.8164409522322 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_9d9391b5-9a9a-46c6-8f29-52a94c9a3c52">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716938772042816 139.81644023186524 -0.02600043326242485 35.716924685818086 139.81639414199563 -0.13786133488567665 35.7169334684518 139.8164389195677 -0.036000433970230006 35.716938772042816 139.81644023186524 -0.02600043326242485</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_5a08aaca-ffdc-4606-a5db-e82bb021c5a5">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.7169334684518 139.8164389195677 -0.036000433970230006 35.716924685818086 139.81639414199563 -0.13786133488567665 35.71692909072055 139.81643763110807 -0.04266710110876677 35.7169334684518 139.8164389195677 -0.036000433970230006</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_324d7719-cb53-40f0-b05b-3e65411972f3">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71692909072055 139.81643763110807 -0.04266710110876677 35.716924685818086 139.81639414199563 -0.13786133488567665 35.71691957007114 139.81643408705636 -0.05543142700976097 35.71692909072055 139.81643763110807 -0.04266710110876677</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_4b1d6328-6f42-4ddf-8f26-1d3ad6b88931">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71691957007114 139.81643408705636 -0.05543142700976097 35.716924685818086 139.81639414199563 -0.13786133488567665 35.71692401450638 139.81639411707798 -0.13800208050383844 35.71691957007114 139.81643408705636 -0.05543142700976097</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_cc781684-f15d-47ec-a07f-6d44b595ce75">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71691957007114 139.81643408705636 -0.05543142700976097 35.71692401450638 139.81639411707798 -0.13800208050383844 35.71690569395532 139.81641840760707 -0.08463302587251621 35.71691957007114 139.81643408705636 -0.05543142700976097</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:trafficArea>
				<tran:TrafficArea gml:id="tfa_7f02eb80-7659-4b9a-b4a4-b7fab9f9e635">
					<core:creationDate>2025-03-21</core:creationDate>
					<tran:function codeSpace="../../codelists/TrafficArea_function.xml">2000</tran:function>
					<tran:lod3MultiSurface>
						<gml:MultiSurface>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_61242337-9389-4536-a051-4a568ed6a2fe">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71694400942787 139.81646170255922 -0.004 35.716943164016875 139.81646709671864 0.0069 35.716947391328944 139.8164615542665 0.0069 35.71694400942787 139.81646170255922 -0.004</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_d0bfb308-4656-4e17-9576-3efc6278b4cf">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716943164016875 139.81646709671864 0.0069 35.71694400942787 139.81646170255922 -0.004 35.71694145955276 139.81646518215018 -0.0040263885589877195 35.716943164016875 139.81646709671864 0.0069</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_62aa8118-94b8-45de-adc9-8a1d748c8f77">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71694400942787 139.81646170255922 -0.004 35.716947391328944 139.8164615542665 0.0069 35.71694866811655 139.81645529892856 -0.03433376718559581 35.71694400942787 139.81646170255922 -0.004</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_1a1c948a-d52b-4f4e-8b23-f3844ec3c8de">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71694866811655 139.81645529892856 -0.03433376718559581 35.716947391328944 139.8164615542665 0.0069 35.71695396793223 139.81646911261706 0.0069 35.71694866811655 139.81645529892856 -0.03433376718559581</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_031ae7ef-7aad-48f3-bf74-fdd320f2ab00">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71694866811655 139.81645529892856 -0.03433376718559581 35.71695396793223 139.81646911261706 0.0069 35.71698893386227 139.8164584492967 -0.05188638116543241 35.71694866811655 139.81645529892856 -0.03433376718559581</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_9431eae0-1164-4a61-b393-fcc29e6d5cfe">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.71698893386227 139.8164584492967 -0.05188638116543241 35.71695396793223 139.81646911261706 0.0069 35.716989427315994 139.81646209447547 0.0285003016367682 35.71698893386227 139.8164584492967 -0.05188638116543241</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_df59810f-0c18-447c-8039-99ddd80d6807">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716989427315994 139.81646209447547 0.0285003016367682 35.71695396793223 139.81646911261706 0.0069 35.716965919663686 139.8164932623662 0.033941590696043744 35.716989427315994 139.81646209447547 0.0285003016367682</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
							<gml:surfaceMember>
								<gml:Polygon gml:id="poly_c6e8c1cc-7bf5-4643-bef2-177358b27bd6">
									<gml:exterior>
										<gml:LinearRing>
											<gml:posList>35.716965919663686 139.8164932623662 0.033941590696043744 35.71695396793223 139.81646911261706 0.0069 35.71694971912456 139.81647468325176 0.0069 35.716965919663686 139.8164932623662 0.033941590696043744</gml:posList>
										</gml:LinearRing>
									</gml:exterior>
								</gml:Polygon>
							</gml:surfaceMember>
						</gml:MultiSurface>
					</tran:lod3MultiSurface>
				</tran:TrafficArea>
			</tran:trafficArea>
			<tran:lod1MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:Polygon>
							<gml:exterior>
								<gml:LinearRing>
									<gml:posList>35.71694145955275 139.81646518215018 0 35.71694316430513 139.8164670972215 0 35.716947390936895 139.81646155451355 0 35.71695396836599 139.81646911316562 0 35.71694971920359 139.81647468351008 0 35.716965919359865 139.81649326187963 0 35.71698942713863 139.81646209497148 0 35.71698893357997 139.81645844879213 0 35.71698768961414 139.81645145048947 0 35.7169918359152 139.81645156014184 0 35.71699254380862 139.81639610641983 0 35.71696697198684 139.81639571172374 0 35.71692468596133 139.81639414234883 0 35.71692401443976 139.81639411706976 0 35.716905693977516 139.81641840810886 0 35.71690299398662 139.81642198753755 0 35.71694145955275 139.81646518215018 0</gml:posList>
								</gml:LinearRing>
							</gml:exterior>
						</gml:Polygon>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod1MultiSurface>
			<tran:lod3MultiSurface>
				<gml:MultiSurface>
					<gml:surfaceMember>
						<gml:CompositeSurface>
							<gml:surfaceMember xlink:href="#poly_12ee9405-b94b-4412-b704-df94754805f6"/>
							<gml:surfaceMember xlink:href="#poly_07e5ab18-53b9-4762-884a-2068e10a04d7"/>
							<gml:surfaceMember xlink:href="#poly_e906f28c-6c46-458b-9184-cf62fccf0985"/>
							<gml:surfaceMember xlink:href="#poly_927a6159-ef80-441b-b2aa-e316bd9b23ca"/>
							<gml:surfaceMember xlink:href="#poly_1a0ecb5b-b384-4232-9090-83d5f79aa0b7"/>
							<gml:surfaceMember xlink:href="#poly_c64147e8-2d5d-42a8-9bac-9e78966b50e5"/>
							<gml:surfaceMember xlink:href="#poly_7525271a-e9a6-4140-9cd9-152c26cdecba"/>
							<gml:surfaceMember xlink:href="#poly_f786cd25-51c7-4b7a-b240-6f849fda995a"/>
							<gml:surfaceMember xlink:href="#poly_46364b7a-9ad6-4f6a-9d0e-32b2f400c8a6"/>
							<gml:surfaceMember xlink:href="#poly_7e97757a-97de-4dff-ab9f-518c36d364a7"/>
							<gml:surfaceMember xlink:href="#poly_dd61294f-2e53-4428-a461-79b27f67c06b"/>
							<gml:surfaceMember xlink:href="#poly_f0bf9674-e4e1-4e29-a8e5-c0e61577f3c2"/>
							<gml:surfaceMember xlink:href="#poly_7d53fa22-bd48-4762-94d0-60a54a239757"/>
							<gml:surfaceMember xlink:href="#poly_06f1fd2b-cc3f-4b73-8933-a662b33b7c4b"/>
							<gml:surfaceMember xlink:href="#poly_27d3a19d-f179-45d6-9e65-792c7c1b5f21"/>
							<gml:surfaceMember xlink:href="#poly_32d26510-6b43-4a2a-86e6-34727747b531"/>
							<gml:surfaceMember xlink:href="#poly_15208d6a-3b70-4bde-943d-36602c0a49b9"/>
							<gml:surfaceMember xlink:href="#poly_019f9973-3b59-4aee-a64d-812bc671fa54"/>
							<gml:surfaceMember xlink:href="#poly_91bb7be8-9e87-4a3b-a157-102fcc278bbe"/>
							<gml:surfaceMember xlink:href="#poly_a135c773-aa7b-4e4c-9e2b-7a46835168da"/>
							<gml:surfaceMember xlink:href="#poly_8a54b23f-d742-4a8a-aab2-dbcdc28e5d01"/>
							<gml:surfaceMember xlink:href="#poly_9d9391b5-9a9a-46c6-8f29-52a94c9a3c52"/>
							<gml:surfaceMember xlink:href="#poly_5a08aaca-ffdc-4606-a5db-e82bb021c5a5"/>
							<gml:surfaceMember xlink:href="#poly_324d7719-cb53-40f0-b05b-3e65411972f3"/>
							<gml:surfaceMember xlink:href="#poly_4b1d6328-6f42-4ddf-8f26-1d3ad6b88931"/>
							<gml:surfaceMember xlink:href="#poly_cc781684-f15d-47ec-a07f-6d44b595ce75"/>
							<gml:surfaceMember xlink:href="#poly_61242337-9389-4536-a051-4a568ed6a2fe"/>
							<gml:surfaceMember xlink:href="#poly_d0bfb308-4656-4e17-9576-3efc6278b4cf"/>
							<gml:surfaceMember xlink:href="#poly_62aa8118-94b8-45de-adc9-8a1d748c8f77"/>
							<gml:surfaceMember xlink:href="#poly_1a1c948a-d52b-4f4e-8b23-f3844ec3c8de"/>
							<gml:surfaceMember xlink:href="#poly_031ae7ef-7aad-48f3-bf74-fdd320f2ab00"/>
							<gml:surfaceMember xlink:href="#poly_9431eae0-1164-4a61-b393-fcc29e6d5cfe"/>
							<gml:surfaceMember xlink:href="#poly_df59810f-0c18-447c-8039-99ddd80d6807"/>
							<gml:surfaceMember xlink:href="#poly_c6e8c1cc-7bf5-4643-bef2-177358b27bd6"/>
						</gml:CompositeSurface>
					</gml:surfaceMember>
				</gml:MultiSurface>
			</tran:lod3MultiSurface>
			<uro:tranDataQualityAttribute>
				<uro:DataQualityAttribute>
					<uro:geometrySrcDescLod1 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod1>
					<uro:geometrySrcDescLod2 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">999</uro:geometrySrcDescLod2>
					<uro:geometrySrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod3>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">023</uro:thematicSrcDesc>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">400</uro:thematicSrcDesc>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">000</uro:thematicSrcDesc>
					<uro:appearanceSrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_appearanceSrcDesc.xml">5</uro:appearanceSrcDescLod3>
					<uro:lodType codeSpace="../../codelists/Road_lodType.xml">3.2</uro:lodType>
					<uro:publicSurveyDataQualityAttribute>
						<uro:PublicSurveyDataQualityAttribute>
							<uro:srcScaleLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod1>
							<uro:srcScaleLod3 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">3</uro:srcScaleLod3>
							<uro:publicSurveySrcDescLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">011</uro:publicSurveySrcDescLod1>
							<uro:publicSurveySrcDescLod3 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">011</uro:publicSurveySrcDescLod3>
						</uro:PublicSurveyDataQualityAttribute>
					</uro:publicSurveyDataQualityAttribute>
				</uro:DataQualityAttribute>
			</uro:tranDataQualityAttribute>
			<uro:roadStructureAttribute>
				<uro:RoadStructureAttribute>
					<uro:sectionType codeSpace="../../codelists/RoadStructureAttribute_sectionType.xml">4</uro:sectionType>
				</uro:RoadStructureAttribute>
			</uro:roadStructureAttribute>
		</tran:Road>
	</core:cityObjectMember>
	
<core:cityObjectMember xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:gml="http://www.opengis.net/gml" xmlns:tran="http://www.opengis.net/citygml/transportation/2.0" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<tran:Road gml:id="tran_4427beea-4e44-4d3a-9271-fde5c885b0fc">
<core:creationDate>2024-03-15</core:creationDate>
<tran:class codeSpace="../../codelists/TransportationComplex_class.xml">1040</tran:class>
	<tran:function codeSpace="../../codelists/Road_function.xml">9020</tran:function>
	<tran:usage codeSpace="../../codelists/Road_usage.xml">9</tran:usage>
<tran:lod1MultiSurface>
<gml:MultiSurface srsName="http://www.opengis.net/def/crs/EPSG/0/6697" srsDimension="3">
<gml:surfaceMember>
<gml:Polygon>
<gml:exterior>
<gml:LinearRing>
<gml:posList>35.73687912151461 139.83082544113662 0 35.73681292989263 139.8309183848169 0 35.73692875423323 139.8308755967368 0 35.73687912151461 139.83082544113662 0</gml:posList>
</gml:LinearRing>
</gml:exterior>
</gml:Polygon>
</gml:surfaceMember>
</gml:MultiSurface>
</tran:lod1MultiSurface>
			<uro:tranDataQualityAttribute>
				<uro:DataQualityAttribute>
					<uro:geometrySrcDescLod1 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">000</uro:geometrySrcDescLod1>
					<uro:geometrySrcDescLod2 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">999</uro:geometrySrcDescLod2>
					<uro:geometrySrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_geometrySrcDesc.xml">999</uro:geometrySrcDescLod3>
					<uro:thematicSrcDesc codeSpace="../../codelists/DataQualityAttribute_thematicSrcDesc.xml">023</uro:thematicSrcDesc>
					<uro:appearanceSrcDescLod3 codeSpace="../../codelists/DataQualityAttribute_appearanceSrcDesc.xml">99</uro:appearanceSrcDescLod3>
					<uro:publicSurveyDataQualityAttribute>
						<uro:PublicSurveyDataQualityAttribute>
							<uro:srcScaleLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_srcScale.xml">1</uro:srcScaleLod1>
							<uro:publicSurveySrcDescLod1 codeSpace="../../codelists/PublicSurveyDataQualityAttribute_publicSurveySrcDesc.xml">023</uro:publicSurveySrcDescLod1>
						</uro:PublicSurveyDataQualityAttribute>
					</uro:publicSurveyDataQualityAttribute>
				</uro:DataQualityAttribute>
			</uro:tranDataQualityAttribute>
<uro:roadStructureAttribute>
<uro:RoadStructureAttribute>
<uro:sectionType codeSpace="../../codelists/RoadStructureAttribute_sectionType.xml">1</uro:sectionType>
</uro:RoadStructureAttribute>
</uro:roadStructureAttribute>
</tran:Road>
</core:cityObjectMember>

</core:CityModel>
