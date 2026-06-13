<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.2" name="oathstar-starter-16x16" tilewidth="16" tileheight="16" tilecount="12" columns="4">
 <image source="oathstar-starter-16x16.png" width="64" height="48"/>
 <tile id="0">
  <properties>
   <property name="name" value="shadow_void"/>
   <property name="color" value="#101419"/>
   <property name="tags" value="terrain,void"/>
   <property name="collision" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="1">
  <properties>
   <property name="name" value="stone_floor"/>
   <property name="color" value="#696f68"/>
   <property name="tags" value="terrain,floor,city"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="2">
  <properties>
   <property name="name" value="wall_face"/>
   <property name="color" value="#434a4a"/>
   <property name="tags" value="wall"/>
   <property name="collision" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="3">
  <properties>
   <property name="name" value="spawn_marker"/>
   <property name="color" value="#5fccb1"/>
   <property name="tags" value="feature,spawn"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="4">
  <properties>
   <property name="name" value="grass"/>
   <property name="color" value="#347b41"/>
   <property name="tags" value="terrain,floor,forest"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="5">
  <properties>
   <property name="name" value="dirt"/>
   <property name="color" value="#815b30"/>
   <property name="tags" value="terrain,floor,road"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="6">
  <properties>
   <property name="name" value="cave_floor"/>
   <property name="color" value="#463e4e"/>
   <property name="tags" value="terrain,floor,cave"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="7">
  <properties>
   <property name="name" value="deep_water"/>
   <property name="color" value="#265c91"/>
   <property name="tags" value="terrain,water"/>
   <property name="collision" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="8">
  <properties>
   <property name="name" value="stairs_up"/>
   <property name="color" value="#a96a37"/>
   <property name="tags" value="feature,stairs"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="9">
  <properties>
   <property name="name" value="stairs_down"/>
   <property name="color" value="#472d1c"/>
   <property name="tags" value="feature,stairs"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
 <tile id="10">
  <properties>
   <property name="name" value="exit_marker"/>
   <property name="color" value="#d8a941"/>
   <property name="tags" value="feature,exit"/>
   <property name="collision" type="bool" value="false"/>
  </properties>
 </tile>
</tileset>
