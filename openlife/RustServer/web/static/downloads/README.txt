Open Life Reborn — early alpha Windows client v0.2.0
====================================================

This is an **early alpha** of the new Rust client. It is experimental.
The **vanilla One Hour One Life client** still works on this server and is
the supported way to play if the alpha client misbehaves.

Server (custom / LAN):
  host  159.195.73.142
  port  8005
  web   http://159.195.73.142:8080/

How to run
----------
1. You still need One Hour One Life / Open Life **game data** (the objects/
   folder). Point at it with the environment variable OHOL_CONTENT_DIR, or
   put a OneLifeData7 folder next to ohol-client.exe.
2. Double-click start-client.bat (or run ohol-client.exe).
3. On the login screen pick the Rust server if it is not already selected,
   then Connect.

This build is Windows-only. Do not commit account keys or .env files.
