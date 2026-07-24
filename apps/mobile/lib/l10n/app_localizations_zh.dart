// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appTitle => 'AnyLive';

  @override
  String appTitleFlavor(String flavor) {
    return 'AnyLive（$flavor）';
  }

  @override
  String get appTagline => 'AnyLive 移动端';

  @override
  String envFlavorLine(String env, String flavor) {
    return '环境：$env · $flavor';
  }

  @override
  String envLine(String env) {
    return '环境：$env';
  }

  @override
  String flavorLine(String flavor) {
    return '渠道：$flavor';
  }

  @override
  String apiLine(String url) {
    return 'API：$url';
  }

  @override
  String brandFlavor(String flavor) {
    return 'AnyLive · $flavor';
  }

  @override
  String get navHome => '首页';

  @override
  String get navFollowing => '关注';

  @override
  String get navGoLive => '开播';

  @override
  String get goLiveAction => '开播';

  @override
  String get navYou => '我的';

  @override
  String get signIn => '登录';

  @override
  String get language => '语言';

  @override
  String get languageSystem => '跟随系统';

  @override
  String get languageChinese => '中文';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageSection => '语言与显示';

  @override
  String get loginTitle => 'AnyLive 登录';

  @override
  String get loginHeadline => '邮箱登录';

  @override
  String get loginDevOtpHint => '本地联调：ALLOW_DEV_OTP=1 时固定验证码 123456。';

  @override
  String get loginOtpHint => '我们会发送一次性验证码到邮箱，若较慢请检查垃圾箱。';

  @override
  String get email => '邮箱';

  @override
  String get otpCode => '验证码';

  @override
  String get otpCodeDev => '验证码（开发：123456）';

  @override
  String get sendOtp => '发送验证码';

  @override
  String get resendCode => '重新发送';

  @override
  String resendInSeconds(int seconds) {
    return '$seconds 秒后可重发';
  }

  @override
  String get ageConfirm => '我确认已满 18 岁';

  @override
  String get privacyAccept => '我接受隐私政策';

  @override
  String get pleaseWait => '请稍候…';

  @override
  String get verifyContinue => '验证并继续';

  @override
  String get useDifferentEmail => '使用其他邮箱';

  @override
  String get privacyPolicy => '隐私政策';

  @override
  String get termsOfService => '服务条款';

  @override
  String get enterValidEmail => '请输入有效邮箱地址。';

  @override
  String get enterOtpFromEmail => '请输入邮箱中的验证码。';

  @override
  String get errInvalidRequest => '请求无效，请检查邮箱和验证码。';

  @override
  String get errInvalidOrExpiredOtp => '验证码无效或已过期，请重新获取。';

  @override
  String get errOtpNotFound => '未找到验证码，请先发送。';

  @override
  String get errTooManyAttempts => '尝试次数过多，请稍后再试。';

  @override
  String errCannotReachApi(String url) {
    return '无法连接 API（$url）。';
  }

  @override
  String get youTitle => '我的';

  @override
  String signedInAs(String name) {
    return '已登录：$name';
  }

  @override
  String get sectionAccount => '账号';

  @override
  String get sectionSession => '会话';

  @override
  String get sectionAbout => '关于';

  @override
  String get profilePrivacy => '资料与隐私';

  @override
  String get profilePrivacySub => '修改昵称、隐私、导出 / 注销';

  @override
  String get wallet => '钱包';

  @override
  String get walletSub => '余额、充值、流水';

  @override
  String get logout => '退出登录';

  @override
  String get logoutSub => '撤销本设备会话';

  @override
  String get logoutConfirmTitle => '退出登录？';

  @override
  String get logoutConfirmBody => '退出后需再次使用邮箱验证码登录。';

  @override
  String get cancel => '取消';

  @override
  String get logOut => '退出';

  @override
  String get editProfile => '编辑资料';

  @override
  String get displayName => '昵称';

  @override
  String get displayNameRequired => '请填写昵称';

  @override
  String get avatarUrl => '头像 URL';

  @override
  String get regionHint => '地区（如 US、SG）';

  @override
  String get save => '保存';

  @override
  String get saving => '保存中…';

  @override
  String get profileSaved => '资料已保存';

  @override
  String get avatarUrlSet => '头像 URL 已更新';

  @override
  String get creatorCenter => '创作者中心';

  @override
  String get loadingStats => '加载统计…';

  @override
  String get followers => '粉丝';

  @override
  String get following => '关注中';

  @override
  String get liveRooms => '直播间';

  @override
  String get totalRooms => '房间总数';

  @override
  String get giftCoins => '礼物币';

  @override
  String get giftCredits => '礼物积分';

  @override
  String get yourRooms => '我的房间';

  @override
  String get privacyData => '隐私与数据';

  @override
  String get exportMyData => '导出我的数据';

  @override
  String exportCopied(int chars) {
    return '已复制导出内容（$chars 字符）';
  }

  @override
  String get signOutAllDevices => '退出全部设备';

  @override
  String signOutAllDevicesCount(int count) {
    return '退出全部设备（$count）';
  }

  @override
  String get signOutAllConfirmTitle => '退出全部设备？';

  @override
  String get signOutAllConfirmBody => '将撤销所有刷新会话，本设备也需要重新登录。';

  @override
  String get signOutAll => '全部退出';

  @override
  String revokedSessions(int count) {
    return '已撤销 $count 个会话';
  }

  @override
  String get sessionRevoked => '会话已撤销';

  @override
  String get activeSessions => '活跃会话';

  @override
  String get activeSessionsHint => '每行对应一个刷新令牌，撤销即结束该设备。';

  @override
  String get revoke => '撤销';

  @override
  String get deleteAccount => '注销账号';

  @override
  String get deleteAccountConfirmTitle => '注销账号？';

  @override
  String get deleteAccountConfirmBody => '将软删除账号，并立即退出登录。';

  @override
  String get delete => '删除';

  @override
  String get close => '关闭';

  @override
  String get retry => '重试';

  @override
  String get copy => '复制';

  @override
  String get submit => '提交';

  @override
  String get start => '开始';

  @override
  String get invite => '邀请';

  @override
  String get more => '更多';

  @override
  String get reason => '原因';

  @override
  String get refresh => '刷新';

  @override
  String balance(int amount) {
    return '余额：$amount';
  }

  @override
  String get recentLedger => '最近流水';

  @override
  String get noLedgerEntries => '暂无流水';

  @override
  String get ledgerUnavailable => '流水暂不可用';

  @override
  String get coinPackages => '金币套餐';

  @override
  String get noPayProducts => '暂无支付商品（需开启 PAY_CHANNELS=mock）。';

  @override
  String get buySandbox => '购买（沙箱）';

  @override
  String get mockTopup => '模拟充值 +100';

  @override
  String get toppedUp => '已充值 +100';

  @override
  String creditedCoins(int coins, String orderId, String status) {
    return '已入账 $coins 金币（订单 $orderId，$status）';
  }

  @override
  String get topUp => '充值';

  @override
  String get feedHome => '首页';

  @override
  String get feedFollowing => '关注';

  @override
  String get feedDiscover => '发现';

  @override
  String get feedHot => '热门';

  @override
  String get searchRoomsOrUsers => '搜索房间或用户';

  @override
  String get noHotRooms => '暂无热门直播';

  @override
  String get noFollowingRooms => '关注的人暂未开播';

  @override
  String get browseHome => '去首页看看';

  @override
  String roomStatusLine(String status) {
    return '房间 · $status';
  }

  @override
  String userIdLine(String id) {
    return '用户 · $id';
  }

  @override
  String get goLiveTitle => '开播';

  @override
  String get youAreLive => '正在直播';

  @override
  String get broadcastWithObs => '使用 OBS 推流';

  @override
  String get roomTitle => '房间标题';

  @override
  String get working => '处理中…';

  @override
  String get startLive => '开始直播';

  @override
  String get endLive => '结束直播';

  @override
  String get stopLive => '停止直播';

  @override
  String get openMyRoom => '打开我的房间';

  @override
  String get openRoom => '打开房间';

  @override
  String roomStatus(String status) {
    return '状态：$status';
  }

  @override
  String get obsServer => 'OBS 服务器';

  @override
  String get obsServerCustom => 'OBS 服务器（自定义 RTMP）';

  @override
  String get obsStreamKey => 'OBS 推流密钥';

  @override
  String get server => '服务器';

  @override
  String get streamKey => '推流密钥';

  @override
  String get refreshObsKeys => '刷新 OBS 密钥';

  @override
  String get copyServer => '复制服务器';

  @override
  String get copyKey => '复制密钥';

  @override
  String get copyStreamKey => '复制推流密钥';

  @override
  String get obsInstructions => 'OBS：设置 → 推流 → 服务=自定义。';

  @override
  String get obsKeySeparate => '请分别粘贴服务器与推流密钥，不要把密钥写进服务器地址。';

  @override
  String get liveEnded => '直播已结束';

  @override
  String get starting => '启动中…';

  @override
  String get noLiveRooms => '暂无直播';

  @override
  String roomIdLine(String id) {
    return '房间：$id';
  }

  @override
  String get unavailable => '（不可用）';

  @override
  String get defaultLiveTitle => '我的直播';

  @override
  String get untitledLive => '未命名直播';

  @override
  String get liveBadge => '直播中';

  @override
  String get shareLive => '分享直播';

  @override
  String get linkCopied => '链接已复制';

  @override
  String get copyLink => '复制链接';

  @override
  String get unfollowedHost => '已取消关注主播';

  @override
  String get followingHost => '已关注主播';

  @override
  String get reportSubmitted => '举报已提交';

  @override
  String get reportRoom => '举报房间';

  @override
  String get follow => '关注';

  @override
  String get unfollow => '取消关注';

  @override
  String likeCount(int count) {
    return '点赞（$count）';
  }

  @override
  String get share => '分享';

  @override
  String get report => '举报';

  @override
  String get host => '主播';

  @override
  String get noMessagesYet => '还没有消息';

  @override
  String get saySomething => '说点什么…';

  @override
  String get roomOfflineChatDisabled => '房间离线 — 无法发送聊天';

  @override
  String get streamEndedChatClosed => '直播已结束 — 聊天已关闭';

  @override
  String get recordingEnabled => '已开启录制';

  @override
  String get recordingDisabled => '已关闭录制';

  @override
  String get enableRecording => '开启录制';

  @override
  String get disableRecording => '关闭录制';

  @override
  String get obsPublish => 'OBS 推流信息';

  @override
  String get liveStartedObsHint => '已开播 — 请用 OBS 推流密钥开播';

  @override
  String get liveStoppedIdle => '已停播 — 房间为空闲（未关闭）';

  @override
  String get inviteCohost => '邀请连麦';

  @override
  String get acceptCohost => '接受连麦';

  @override
  String get declineCohost => '拒绝连麦';

  @override
  String get inviteeUserId => '被邀请用户 ID（UUID）';

  @override
  String get startPk => '开始 PK';

  @override
  String get endPk => '结束 PK';

  @override
  String get opponentRoomId => '对手房间 ID（UUID）';

  @override
  String get pkStarted => 'PK 已开始';

  @override
  String get pkEnded => 'PK 已结束';

  @override
  String get pkUnavailable => 'PK 不可用（功能关闭）';

  @override
  String get cohostUnavailable => '连麦不可用（功能关闭）';

  @override
  String get inviteUnavailable => '邀请不可用（功能关闭）';

  @override
  String get livekitJoin => 'LiveKit 加入';

  @override
  String get copyToken => '复制令牌';

  @override
  String get statusEnded => '已结束';

  @override
  String get streamEnded => '直播已结束';

  @override
  String get hostOffline => '主播离线';

  @override
  String get livePlayUrlUnavailable => '直播中 — 播放地址不可用';

  @override
  String get openStreamExternal => '在外部播放器打开直播地址';

  @override
  String get roomForceClosed => '该房间已被强制关闭';

  @override
  String get hostStoppedMayReturn => '主播已停播 — 可能再次开播';

  @override
  String get copiedStreamUrl => '已复制直播地址';

  @override
  String get hlsBrowserHlsJs => 'HLS（浏览器 · hls.js · 静音）';

  @override
  String get hlsBrowserMuted => 'HLS（浏览器 · 静音自动播放）';

  @override
  String get hlsInApp => 'HLS（应用内）';

  @override
  String get hlsStream => 'HLS 直播流';

  @override
  String get playerDisabledCopyUrl => '应用内播放器已关闭 — 请复制地址外部打开';

  @override
  String get browserAutoplayMuted => '浏览器自动播放为静音，请用控件取消静音。';

  @override
  String get copyStreamUrl => '复制直播地址';

  @override
  String get playRetry => '播放 / 重试';

  @override
  String get openingStream => '正在打开直播…';

  @override
  String get buffering => '缓冲中…';

  @override
  String get waitingForVideo => '等待画面…';

  @override
  String get tapToPlay => '点按播放';

  @override
  String actionFailed(String action, String error) {
    return '$action 失败：$error';
  }

  @override
  String cohostInviteSent(String status) {
    return '连麦邀请已发送（$status）';
  }

  @override
  String cohostAccepted(String status) {
    return '已接受连麦（$status）';
  }

  @override
  String cohostDeclined(String status) {
    return '已拒绝连麦（$status）';
  }

  @override
  String pkEndedWithWinner(String winnerRoomId) {
    return 'PK 已结束 · 胜者 $winnerRoomId';
  }

  @override
  String pkScoreLine(String status, int scoreA, int scoreB) {
    return 'PK $status：$scoreA – $scoreB';
  }

  @override
  String pkScoreLineWinner(
    String status,
    int scoreA,
    int scoreB,
    String winnerRoomId,
  ) {
    return 'PK $status：$scoreA – $scoreB · 胜 $winnerRoomId';
  }

  @override
  String livekitJoinDetail(
    String url,
    String room,
    String identity,
    String token,
  ) {
    return 'url: $url\nroom: $room\nidentity: $identity\ntoken: $token';
  }

  @override
  String labelCopied(String label) {
    return '已复制$label';
  }

  @override
  String coinsPriceLine(int coins, String amount, String currency) {
    return '$coins 金币 · $amount $currency';
  }
}
