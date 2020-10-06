Build OpenLife
=============
0. [Download Git](https://git-scm.com/downloads)
1. [Download NodeJS+NPM](https://nodejs.org/en/download/)
2. [Download Hashlink](https://hashlink.haxe.org/#download) (add it to your system PATH) on mac you can do ```brew install hashlink``` on linux you have to build it yourself with make
3. terminal: ```npm install```
4. download OneLifeData, terminal: ```npx haxe setup_data.hxml```
5. build app, terminal: ```npx haxe app.hxml```
6. run app, terminal: ```hl app.hl```
7. first time run it will generate config.json and data.json, these are gitignore files and can be used to set how the app behaves