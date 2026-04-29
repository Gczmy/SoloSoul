1.检查私有库https://github.com/Gczmy/SoloSoul_code.git是否与本地状态相同，是否需要更新。如果需要更新推送，先将本地更新优先推送至私有库。
2.重新编译最新的dmg文件，编译脚本使用flutter/build_dmg.sh。
3.把最新编译的dmg文件移动至/Users/zzc/PycharmProjects/SoloSoul/
4.把最新编译的dmg文件release到公开库https://github.com/Gczmy/SoloSoul.git，版本号使用https://semver.org/lang/zh-CN/定义的规则，延续之前已发布的版本号加一。
5.更新公开库的changelog（简洁版本）。
6.更新私有库的changelog（详细版本）。
