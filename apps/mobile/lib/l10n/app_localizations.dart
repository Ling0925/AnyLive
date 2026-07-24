import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('zh'),
    Locale('en'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In zh, this message translates to:
  /// **'AnyLive'**
  String get appTitle;

  /// No description provided for @appTitleFlavor.
  ///
  /// In zh, this message translates to:
  /// **'AnyLive（{flavor}）'**
  String appTitleFlavor(String flavor);

  /// No description provided for @appTagline.
  ///
  /// In zh, this message translates to:
  /// **'AnyLive 移动端'**
  String get appTagline;

  /// No description provided for @envFlavorLine.
  ///
  /// In zh, this message translates to:
  /// **'环境：{env} · {flavor}'**
  String envFlavorLine(String env, String flavor);

  /// No description provided for @envLine.
  ///
  /// In zh, this message translates to:
  /// **'环境：{env}'**
  String envLine(String env);

  /// No description provided for @flavorLine.
  ///
  /// In zh, this message translates to:
  /// **'渠道：{flavor}'**
  String flavorLine(String flavor);

  /// No description provided for @apiLine.
  ///
  /// In zh, this message translates to:
  /// **'API：{url}'**
  String apiLine(String url);

  /// No description provided for @brandFlavor.
  ///
  /// In zh, this message translates to:
  /// **'AnyLive · {flavor}'**
  String brandFlavor(String flavor);

  /// No description provided for @navHome.
  ///
  /// In zh, this message translates to:
  /// **'首页'**
  String get navHome;

  /// No description provided for @navFollowing.
  ///
  /// In zh, this message translates to:
  /// **'关注'**
  String get navFollowing;

  /// No description provided for @navGoLive.
  ///
  /// In zh, this message translates to:
  /// **'开播'**
  String get navGoLive;

  /// No description provided for @goLiveAction.
  ///
  /// In zh, this message translates to:
  /// **'开播'**
  String get goLiveAction;

  /// No description provided for @navYou.
  ///
  /// In zh, this message translates to:
  /// **'我的'**
  String get navYou;

  /// No description provided for @signIn.
  ///
  /// In zh, this message translates to:
  /// **'登录'**
  String get signIn;

  /// No description provided for @language.
  ///
  /// In zh, this message translates to:
  /// **'语言'**
  String get language;

  /// No description provided for @languageSystem.
  ///
  /// In zh, this message translates to:
  /// **'跟随系统'**
  String get languageSystem;

  /// No description provided for @languageChinese.
  ///
  /// In zh, this message translates to:
  /// **'中文'**
  String get languageChinese;

  /// No description provided for @languageEnglish.
  ///
  /// In zh, this message translates to:
  /// **'English'**
  String get languageEnglish;

  /// No description provided for @languageSection.
  ///
  /// In zh, this message translates to:
  /// **'语言与显示'**
  String get languageSection;

  /// No description provided for @loginTitle.
  ///
  /// In zh, this message translates to:
  /// **'AnyLive 登录'**
  String get loginTitle;

  /// No description provided for @loginHeadline.
  ///
  /// In zh, this message translates to:
  /// **'邮箱登录'**
  String get loginHeadline;

  /// No description provided for @loginDevOtpHint.
  ///
  /// In zh, this message translates to:
  /// **'本地联调：ALLOW_DEV_OTP=1 时固定验证码 123456。'**
  String get loginDevOtpHint;

  /// No description provided for @loginOtpHint.
  ///
  /// In zh, this message translates to:
  /// **'我们会发送一次性验证码到邮箱，若较慢请检查垃圾箱。'**
  String get loginOtpHint;

  /// No description provided for @email.
  ///
  /// In zh, this message translates to:
  /// **'邮箱'**
  String get email;

  /// No description provided for @otpCode.
  ///
  /// In zh, this message translates to:
  /// **'验证码'**
  String get otpCode;

  /// No description provided for @otpCodeDev.
  ///
  /// In zh, this message translates to:
  /// **'验证码（开发：123456）'**
  String get otpCodeDev;

  /// No description provided for @sendOtp.
  ///
  /// In zh, this message translates to:
  /// **'发送验证码'**
  String get sendOtp;

  /// No description provided for @resendCode.
  ///
  /// In zh, this message translates to:
  /// **'重新发送'**
  String get resendCode;

  /// No description provided for @resendInSeconds.
  ///
  /// In zh, this message translates to:
  /// **'{seconds} 秒后可重发'**
  String resendInSeconds(int seconds);

  /// No description provided for @ageConfirm.
  ///
  /// In zh, this message translates to:
  /// **'我确认已满 18 岁'**
  String get ageConfirm;

  /// No description provided for @privacyAccept.
  ///
  /// In zh, this message translates to:
  /// **'我接受隐私政策'**
  String get privacyAccept;

  /// No description provided for @pleaseWait.
  ///
  /// In zh, this message translates to:
  /// **'请稍候…'**
  String get pleaseWait;

  /// No description provided for @verifyContinue.
  ///
  /// In zh, this message translates to:
  /// **'验证并继续'**
  String get verifyContinue;

  /// No description provided for @useDifferentEmail.
  ///
  /// In zh, this message translates to:
  /// **'使用其他邮箱'**
  String get useDifferentEmail;

  /// No description provided for @privacyPolicy.
  ///
  /// In zh, this message translates to:
  /// **'隐私政策'**
  String get privacyPolicy;

  /// No description provided for @termsOfService.
  ///
  /// In zh, this message translates to:
  /// **'服务条款'**
  String get termsOfService;

  /// No description provided for @enterValidEmail.
  ///
  /// In zh, this message translates to:
  /// **'请输入有效邮箱地址。'**
  String get enterValidEmail;

  /// No description provided for @enterOtpFromEmail.
  ///
  /// In zh, this message translates to:
  /// **'请输入邮箱中的验证码。'**
  String get enterOtpFromEmail;

  /// No description provided for @errInvalidRequest.
  ///
  /// In zh, this message translates to:
  /// **'请求无效，请检查邮箱和验证码。'**
  String get errInvalidRequest;

  /// No description provided for @errInvalidOrExpiredOtp.
  ///
  /// In zh, this message translates to:
  /// **'验证码无效或已过期，请重新获取。'**
  String get errInvalidOrExpiredOtp;

  /// No description provided for @errOtpNotFound.
  ///
  /// In zh, this message translates to:
  /// **'未找到验证码，请先发送。'**
  String get errOtpNotFound;

  /// No description provided for @errTooManyAttempts.
  ///
  /// In zh, this message translates to:
  /// **'尝试次数过多，请稍后再试。'**
  String get errTooManyAttempts;

  /// No description provided for @errCannotReachApi.
  ///
  /// In zh, this message translates to:
  /// **'无法连接 API（{url}）。'**
  String errCannotReachApi(String url);

  /// No description provided for @youTitle.
  ///
  /// In zh, this message translates to:
  /// **'我的'**
  String get youTitle;

  /// No description provided for @signedInAs.
  ///
  /// In zh, this message translates to:
  /// **'已登录：{name}'**
  String signedInAs(String name);

  /// No description provided for @sectionAccount.
  ///
  /// In zh, this message translates to:
  /// **'账号'**
  String get sectionAccount;

  /// No description provided for @sectionSession.
  ///
  /// In zh, this message translates to:
  /// **'会话'**
  String get sectionSession;

  /// No description provided for @sectionAbout.
  ///
  /// In zh, this message translates to:
  /// **'关于'**
  String get sectionAbout;

  /// No description provided for @profilePrivacy.
  ///
  /// In zh, this message translates to:
  /// **'资料与隐私'**
  String get profilePrivacy;

  /// No description provided for @profilePrivacySub.
  ///
  /// In zh, this message translates to:
  /// **'修改昵称、隐私、导出 / 注销'**
  String get profilePrivacySub;

  /// No description provided for @wallet.
  ///
  /// In zh, this message translates to:
  /// **'钱包'**
  String get wallet;

  /// No description provided for @walletSub.
  ///
  /// In zh, this message translates to:
  /// **'余额、充值、流水'**
  String get walletSub;

  /// No description provided for @logout.
  ///
  /// In zh, this message translates to:
  /// **'退出登录'**
  String get logout;

  /// No description provided for @logoutSub.
  ///
  /// In zh, this message translates to:
  /// **'撤销本设备会话'**
  String get logoutSub;

  /// No description provided for @logoutConfirmTitle.
  ///
  /// In zh, this message translates to:
  /// **'退出登录？'**
  String get logoutConfirmTitle;

  /// No description provided for @logoutConfirmBody.
  ///
  /// In zh, this message translates to:
  /// **'退出后需再次使用邮箱验证码登录。'**
  String get logoutConfirmBody;

  /// No description provided for @cancel.
  ///
  /// In zh, this message translates to:
  /// **'取消'**
  String get cancel;

  /// No description provided for @logOut.
  ///
  /// In zh, this message translates to:
  /// **'退出'**
  String get logOut;

  /// No description provided for @editProfile.
  ///
  /// In zh, this message translates to:
  /// **'编辑资料'**
  String get editProfile;

  /// No description provided for @displayName.
  ///
  /// In zh, this message translates to:
  /// **'昵称'**
  String get displayName;

  /// No description provided for @displayNameRequired.
  ///
  /// In zh, this message translates to:
  /// **'请填写昵称'**
  String get displayNameRequired;

  /// No description provided for @avatarUrl.
  ///
  /// In zh, this message translates to:
  /// **'头像 URL'**
  String get avatarUrl;

  /// No description provided for @regionHint.
  ///
  /// In zh, this message translates to:
  /// **'地区（如 US、SG）'**
  String get regionHint;

  /// No description provided for @save.
  ///
  /// In zh, this message translates to:
  /// **'保存'**
  String get save;

  /// No description provided for @saving.
  ///
  /// In zh, this message translates to:
  /// **'保存中…'**
  String get saving;

  /// No description provided for @profileSaved.
  ///
  /// In zh, this message translates to:
  /// **'资料已保存'**
  String get profileSaved;

  /// No description provided for @avatarUrlSet.
  ///
  /// In zh, this message translates to:
  /// **'头像 URL 已更新'**
  String get avatarUrlSet;

  /// No description provided for @creatorCenter.
  ///
  /// In zh, this message translates to:
  /// **'创作者中心'**
  String get creatorCenter;

  /// No description provided for @loadingStats.
  ///
  /// In zh, this message translates to:
  /// **'加载统计…'**
  String get loadingStats;

  /// No description provided for @followers.
  ///
  /// In zh, this message translates to:
  /// **'粉丝'**
  String get followers;

  /// No description provided for @following.
  ///
  /// In zh, this message translates to:
  /// **'关注中'**
  String get following;

  /// No description provided for @liveRooms.
  ///
  /// In zh, this message translates to:
  /// **'直播间'**
  String get liveRooms;

  /// No description provided for @totalRooms.
  ///
  /// In zh, this message translates to:
  /// **'房间总数'**
  String get totalRooms;

  /// No description provided for @giftCoins.
  ///
  /// In zh, this message translates to:
  /// **'礼物币'**
  String get giftCoins;

  /// No description provided for @giftCredits.
  ///
  /// In zh, this message translates to:
  /// **'礼物积分'**
  String get giftCredits;

  /// No description provided for @yourRooms.
  ///
  /// In zh, this message translates to:
  /// **'我的房间'**
  String get yourRooms;

  /// No description provided for @privacyData.
  ///
  /// In zh, this message translates to:
  /// **'隐私与数据'**
  String get privacyData;

  /// No description provided for @exportMyData.
  ///
  /// In zh, this message translates to:
  /// **'导出我的数据'**
  String get exportMyData;

  /// No description provided for @exportCopied.
  ///
  /// In zh, this message translates to:
  /// **'已复制导出内容（{chars} 字符）'**
  String exportCopied(int chars);

  /// No description provided for @signOutAllDevices.
  ///
  /// In zh, this message translates to:
  /// **'退出全部设备'**
  String get signOutAllDevices;

  /// No description provided for @signOutAllDevicesCount.
  ///
  /// In zh, this message translates to:
  /// **'退出全部设备（{count}）'**
  String signOutAllDevicesCount(int count);

  /// No description provided for @signOutAllConfirmTitle.
  ///
  /// In zh, this message translates to:
  /// **'退出全部设备？'**
  String get signOutAllConfirmTitle;

  /// No description provided for @signOutAllConfirmBody.
  ///
  /// In zh, this message translates to:
  /// **'将撤销所有刷新会话，本设备也需要重新登录。'**
  String get signOutAllConfirmBody;

  /// No description provided for @signOutAll.
  ///
  /// In zh, this message translates to:
  /// **'全部退出'**
  String get signOutAll;

  /// No description provided for @revokedSessions.
  ///
  /// In zh, this message translates to:
  /// **'已撤销 {count} 个会话'**
  String revokedSessions(int count);

  /// No description provided for @sessionRevoked.
  ///
  /// In zh, this message translates to:
  /// **'会话已撤销'**
  String get sessionRevoked;

  /// No description provided for @activeSessions.
  ///
  /// In zh, this message translates to:
  /// **'活跃会话'**
  String get activeSessions;

  /// No description provided for @activeSessionsHint.
  ///
  /// In zh, this message translates to:
  /// **'每行对应一个刷新令牌，撤销即结束该设备。'**
  String get activeSessionsHint;

  /// No description provided for @revoke.
  ///
  /// In zh, this message translates to:
  /// **'撤销'**
  String get revoke;

  /// No description provided for @deleteAccount.
  ///
  /// In zh, this message translates to:
  /// **'注销账号'**
  String get deleteAccount;

  /// No description provided for @deleteAccountConfirmTitle.
  ///
  /// In zh, this message translates to:
  /// **'注销账号？'**
  String get deleteAccountConfirmTitle;

  /// No description provided for @deleteAccountConfirmBody.
  ///
  /// In zh, this message translates to:
  /// **'将软删除账号，并立即退出登录。'**
  String get deleteAccountConfirmBody;

  /// No description provided for @delete.
  ///
  /// In zh, this message translates to:
  /// **'删除'**
  String get delete;

  /// No description provided for @close.
  ///
  /// In zh, this message translates to:
  /// **'关闭'**
  String get close;

  /// No description provided for @retry.
  ///
  /// In zh, this message translates to:
  /// **'重试'**
  String get retry;

  /// No description provided for @copy.
  ///
  /// In zh, this message translates to:
  /// **'复制'**
  String get copy;

  /// No description provided for @submit.
  ///
  /// In zh, this message translates to:
  /// **'提交'**
  String get submit;

  /// No description provided for @start.
  ///
  /// In zh, this message translates to:
  /// **'开始'**
  String get start;

  /// No description provided for @invite.
  ///
  /// In zh, this message translates to:
  /// **'邀请'**
  String get invite;

  /// No description provided for @more.
  ///
  /// In zh, this message translates to:
  /// **'更多'**
  String get more;

  /// No description provided for @reason.
  ///
  /// In zh, this message translates to:
  /// **'原因'**
  String get reason;

  /// No description provided for @refresh.
  ///
  /// In zh, this message translates to:
  /// **'刷新'**
  String get refresh;

  /// No description provided for @balance.
  ///
  /// In zh, this message translates to:
  /// **'余额：{amount}'**
  String balance(int amount);

  /// No description provided for @recentLedger.
  ///
  /// In zh, this message translates to:
  /// **'最近流水'**
  String get recentLedger;

  /// No description provided for @noLedgerEntries.
  ///
  /// In zh, this message translates to:
  /// **'暂无流水'**
  String get noLedgerEntries;

  /// No description provided for @ledgerUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'流水暂不可用'**
  String get ledgerUnavailable;

  /// No description provided for @coinPackages.
  ///
  /// In zh, this message translates to:
  /// **'金币套餐'**
  String get coinPackages;

  /// No description provided for @noPayProducts.
  ///
  /// In zh, this message translates to:
  /// **'暂无支付商品（需开启 PAY_CHANNELS=mock）。'**
  String get noPayProducts;

  /// No description provided for @buySandbox.
  ///
  /// In zh, this message translates to:
  /// **'购买（沙箱）'**
  String get buySandbox;

  /// No description provided for @mockTopup.
  ///
  /// In zh, this message translates to:
  /// **'模拟充值 +100'**
  String get mockTopup;

  /// No description provided for @toppedUp.
  ///
  /// In zh, this message translates to:
  /// **'已充值 +100'**
  String get toppedUp;

  /// No description provided for @creditedCoins.
  ///
  /// In zh, this message translates to:
  /// **'已入账 {coins} 金币（订单 {orderId}，{status}）'**
  String creditedCoins(int coins, String orderId, String status);

  /// No description provided for @topUp.
  ///
  /// In zh, this message translates to:
  /// **'充值'**
  String get topUp;

  /// No description provided for @feedHome.
  ///
  /// In zh, this message translates to:
  /// **'首页'**
  String get feedHome;

  /// No description provided for @feedFollowing.
  ///
  /// In zh, this message translates to:
  /// **'关注'**
  String get feedFollowing;

  /// No description provided for @feedDiscover.
  ///
  /// In zh, this message translates to:
  /// **'发现'**
  String get feedDiscover;

  /// No description provided for @feedHot.
  ///
  /// In zh, this message translates to:
  /// **'热门'**
  String get feedHot;

  /// No description provided for @searchRoomsOrUsers.
  ///
  /// In zh, this message translates to:
  /// **'搜索房间或用户'**
  String get searchRoomsOrUsers;

  /// No description provided for @noHotRooms.
  ///
  /// In zh, this message translates to:
  /// **'暂无热门直播'**
  String get noHotRooms;

  /// No description provided for @noFollowingRooms.
  ///
  /// In zh, this message translates to:
  /// **'关注的人暂未开播'**
  String get noFollowingRooms;

  /// No description provided for @browseHome.
  ///
  /// In zh, this message translates to:
  /// **'去首页看看'**
  String get browseHome;

  /// No description provided for @roomStatusLine.
  ///
  /// In zh, this message translates to:
  /// **'房间 · {status}'**
  String roomStatusLine(String status);

  /// No description provided for @userIdLine.
  ///
  /// In zh, this message translates to:
  /// **'用户 · {id}'**
  String userIdLine(String id);

  /// No description provided for @goLiveTitle.
  ///
  /// In zh, this message translates to:
  /// **'开播'**
  String get goLiveTitle;

  /// No description provided for @youAreLive.
  ///
  /// In zh, this message translates to:
  /// **'正在直播'**
  String get youAreLive;

  /// No description provided for @broadcastWithObs.
  ///
  /// In zh, this message translates to:
  /// **'使用 OBS 推流'**
  String get broadcastWithObs;

  /// No description provided for @roomTitle.
  ///
  /// In zh, this message translates to:
  /// **'房间标题'**
  String get roomTitle;

  /// No description provided for @working.
  ///
  /// In zh, this message translates to:
  /// **'处理中…'**
  String get working;

  /// No description provided for @startLive.
  ///
  /// In zh, this message translates to:
  /// **'开始直播'**
  String get startLive;

  /// No description provided for @endLive.
  ///
  /// In zh, this message translates to:
  /// **'结束直播'**
  String get endLive;

  /// No description provided for @stopLive.
  ///
  /// In zh, this message translates to:
  /// **'停止直播'**
  String get stopLive;

  /// No description provided for @openMyRoom.
  ///
  /// In zh, this message translates to:
  /// **'打开我的房间'**
  String get openMyRoom;

  /// No description provided for @openRoom.
  ///
  /// In zh, this message translates to:
  /// **'打开房间'**
  String get openRoom;

  /// No description provided for @roomStatus.
  ///
  /// In zh, this message translates to:
  /// **'状态：{status}'**
  String roomStatus(String status);

  /// No description provided for @obsServer.
  ///
  /// In zh, this message translates to:
  /// **'OBS 服务器'**
  String get obsServer;

  /// No description provided for @obsServerCustom.
  ///
  /// In zh, this message translates to:
  /// **'OBS 服务器（自定义 RTMP）'**
  String get obsServerCustom;

  /// No description provided for @obsStreamKey.
  ///
  /// In zh, this message translates to:
  /// **'OBS 推流密钥'**
  String get obsStreamKey;

  /// No description provided for @server.
  ///
  /// In zh, this message translates to:
  /// **'服务器'**
  String get server;

  /// No description provided for @streamKey.
  ///
  /// In zh, this message translates to:
  /// **'推流密钥'**
  String get streamKey;

  /// No description provided for @refreshObsKeys.
  ///
  /// In zh, this message translates to:
  /// **'刷新 OBS 密钥'**
  String get refreshObsKeys;

  /// No description provided for @copyServer.
  ///
  /// In zh, this message translates to:
  /// **'复制服务器'**
  String get copyServer;

  /// No description provided for @copyKey.
  ///
  /// In zh, this message translates to:
  /// **'复制密钥'**
  String get copyKey;

  /// No description provided for @copyStreamKey.
  ///
  /// In zh, this message translates to:
  /// **'复制推流密钥'**
  String get copyStreamKey;

  /// No description provided for @obsInstructions.
  ///
  /// In zh, this message translates to:
  /// **'OBS：设置 → 推流 → 服务=自定义。'**
  String get obsInstructions;

  /// No description provided for @obsKeySeparate.
  ///
  /// In zh, this message translates to:
  /// **'请分别粘贴服务器与推流密钥，不要把密钥写进服务器地址。'**
  String get obsKeySeparate;

  /// No description provided for @liveEnded.
  ///
  /// In zh, this message translates to:
  /// **'直播已结束'**
  String get liveEnded;

  /// No description provided for @starting.
  ///
  /// In zh, this message translates to:
  /// **'启动中…'**
  String get starting;

  /// No description provided for @noLiveRooms.
  ///
  /// In zh, this message translates to:
  /// **'暂无直播'**
  String get noLiveRooms;

  /// No description provided for @roomIdLine.
  ///
  /// In zh, this message translates to:
  /// **'房间：{id}'**
  String roomIdLine(String id);

  /// No description provided for @unavailable.
  ///
  /// In zh, this message translates to:
  /// **'（不可用）'**
  String get unavailable;

  /// No description provided for @defaultLiveTitle.
  ///
  /// In zh, this message translates to:
  /// **'我的直播'**
  String get defaultLiveTitle;

  /// No description provided for @untitledLive.
  ///
  /// In zh, this message translates to:
  /// **'未命名直播'**
  String get untitledLive;

  /// No description provided for @liveBadge.
  ///
  /// In zh, this message translates to:
  /// **'直播中'**
  String get liveBadge;

  /// No description provided for @shareLive.
  ///
  /// In zh, this message translates to:
  /// **'分享直播'**
  String get shareLive;

  /// No description provided for @linkCopied.
  ///
  /// In zh, this message translates to:
  /// **'链接已复制'**
  String get linkCopied;

  /// No description provided for @copyLink.
  ///
  /// In zh, this message translates to:
  /// **'复制链接'**
  String get copyLink;

  /// No description provided for @unfollowedHost.
  ///
  /// In zh, this message translates to:
  /// **'已取消关注主播'**
  String get unfollowedHost;

  /// No description provided for @followingHost.
  ///
  /// In zh, this message translates to:
  /// **'已关注主播'**
  String get followingHost;

  /// No description provided for @reportSubmitted.
  ///
  /// In zh, this message translates to:
  /// **'举报已提交'**
  String get reportSubmitted;

  /// No description provided for @reportRoom.
  ///
  /// In zh, this message translates to:
  /// **'举报房间'**
  String get reportRoom;

  /// No description provided for @follow.
  ///
  /// In zh, this message translates to:
  /// **'关注'**
  String get follow;

  /// No description provided for @unfollow.
  ///
  /// In zh, this message translates to:
  /// **'取消关注'**
  String get unfollow;

  /// No description provided for @likeCount.
  ///
  /// In zh, this message translates to:
  /// **'点赞（{count}）'**
  String likeCount(int count);

  /// No description provided for @share.
  ///
  /// In zh, this message translates to:
  /// **'分享'**
  String get share;

  /// No description provided for @report.
  ///
  /// In zh, this message translates to:
  /// **'举报'**
  String get report;

  /// No description provided for @host.
  ///
  /// In zh, this message translates to:
  /// **'主播'**
  String get host;

  /// No description provided for @noMessagesYet.
  ///
  /// In zh, this message translates to:
  /// **'还没有消息'**
  String get noMessagesYet;

  /// No description provided for @saySomething.
  ///
  /// In zh, this message translates to:
  /// **'说点什么…'**
  String get saySomething;

  /// No description provided for @roomOfflineChatDisabled.
  ///
  /// In zh, this message translates to:
  /// **'房间离线 — 无法发送聊天'**
  String get roomOfflineChatDisabled;

  /// No description provided for @streamEndedChatClosed.
  ///
  /// In zh, this message translates to:
  /// **'直播已结束 — 聊天已关闭'**
  String get streamEndedChatClosed;

  /// No description provided for @recordingEnabled.
  ///
  /// In zh, this message translates to:
  /// **'已开启录制'**
  String get recordingEnabled;

  /// No description provided for @recordingDisabled.
  ///
  /// In zh, this message translates to:
  /// **'已关闭录制'**
  String get recordingDisabled;

  /// No description provided for @enableRecording.
  ///
  /// In zh, this message translates to:
  /// **'开启录制'**
  String get enableRecording;

  /// No description provided for @disableRecording.
  ///
  /// In zh, this message translates to:
  /// **'关闭录制'**
  String get disableRecording;

  /// No description provided for @obsPublish.
  ///
  /// In zh, this message translates to:
  /// **'OBS 推流信息'**
  String get obsPublish;

  /// No description provided for @liveStartedObsHint.
  ///
  /// In zh, this message translates to:
  /// **'已开播 — 请用 OBS 推流密钥开播'**
  String get liveStartedObsHint;

  /// No description provided for @liveStoppedIdle.
  ///
  /// In zh, this message translates to:
  /// **'已停播 — 房间为空闲（未关闭）'**
  String get liveStoppedIdle;

  /// No description provided for @inviteCohost.
  ///
  /// In zh, this message translates to:
  /// **'邀请连麦'**
  String get inviteCohost;

  /// No description provided for @acceptCohost.
  ///
  /// In zh, this message translates to:
  /// **'接受连麦'**
  String get acceptCohost;

  /// No description provided for @declineCohost.
  ///
  /// In zh, this message translates to:
  /// **'拒绝连麦'**
  String get declineCohost;

  /// No description provided for @inviteeUserId.
  ///
  /// In zh, this message translates to:
  /// **'被邀请用户 ID（UUID）'**
  String get inviteeUserId;

  /// No description provided for @startPk.
  ///
  /// In zh, this message translates to:
  /// **'开始 PK'**
  String get startPk;

  /// No description provided for @endPk.
  ///
  /// In zh, this message translates to:
  /// **'结束 PK'**
  String get endPk;

  /// No description provided for @opponentRoomId.
  ///
  /// In zh, this message translates to:
  /// **'对手房间 ID（UUID）'**
  String get opponentRoomId;

  /// No description provided for @pkStarted.
  ///
  /// In zh, this message translates to:
  /// **'PK 已开始'**
  String get pkStarted;

  /// No description provided for @pkEnded.
  ///
  /// In zh, this message translates to:
  /// **'PK 已结束'**
  String get pkEnded;

  /// No description provided for @pkUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'PK 不可用（功能关闭）'**
  String get pkUnavailable;

  /// No description provided for @cohostUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'连麦不可用（功能关闭）'**
  String get cohostUnavailable;

  /// No description provided for @inviteUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'邀请不可用（功能关闭）'**
  String get inviteUnavailable;

  /// No description provided for @livekitJoin.
  ///
  /// In zh, this message translates to:
  /// **'LiveKit 加入'**
  String get livekitJoin;

  /// No description provided for @copyToken.
  ///
  /// In zh, this message translates to:
  /// **'复制令牌'**
  String get copyToken;

  /// No description provided for @statusEnded.
  ///
  /// In zh, this message translates to:
  /// **'已结束'**
  String get statusEnded;

  /// No description provided for @streamEnded.
  ///
  /// In zh, this message translates to:
  /// **'直播已结束'**
  String get streamEnded;

  /// No description provided for @hostOffline.
  ///
  /// In zh, this message translates to:
  /// **'主播离线'**
  String get hostOffline;

  /// No description provided for @livePlayUrlUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'直播中 — 播放地址不可用'**
  String get livePlayUrlUnavailable;

  /// No description provided for @openStreamExternal.
  ///
  /// In zh, this message translates to:
  /// **'在外部播放器打开直播地址'**
  String get openStreamExternal;

  /// No description provided for @roomForceClosed.
  ///
  /// In zh, this message translates to:
  /// **'该房间已被强制关闭'**
  String get roomForceClosed;

  /// No description provided for @hostStoppedMayReturn.
  ///
  /// In zh, this message translates to:
  /// **'主播已停播 — 可能再次开播'**
  String get hostStoppedMayReturn;

  /// No description provided for @copiedStreamUrl.
  ///
  /// In zh, this message translates to:
  /// **'已复制直播地址'**
  String get copiedStreamUrl;

  /// No description provided for @hlsBrowserHlsJs.
  ///
  /// In zh, this message translates to:
  /// **'HLS（浏览器 · hls.js · 静音）'**
  String get hlsBrowserHlsJs;

  /// No description provided for @hlsBrowserMuted.
  ///
  /// In zh, this message translates to:
  /// **'HLS（浏览器 · 静音自动播放）'**
  String get hlsBrowserMuted;

  /// No description provided for @hlsInApp.
  ///
  /// In zh, this message translates to:
  /// **'HLS（应用内）'**
  String get hlsInApp;

  /// No description provided for @hlsStream.
  ///
  /// In zh, this message translates to:
  /// **'HLS 直播流'**
  String get hlsStream;

  /// No description provided for @playerDisabledCopyUrl.
  ///
  /// In zh, this message translates to:
  /// **'应用内播放器已关闭 — 请复制地址外部打开'**
  String get playerDisabledCopyUrl;

  /// No description provided for @browserAutoplayMuted.
  ///
  /// In zh, this message translates to:
  /// **'浏览器自动播放为静音，请用控件取消静音。'**
  String get browserAutoplayMuted;

  /// No description provided for @copyStreamUrl.
  ///
  /// In zh, this message translates to:
  /// **'复制直播地址'**
  String get copyStreamUrl;

  /// No description provided for @playRetry.
  ///
  /// In zh, this message translates to:
  /// **'播放 / 重试'**
  String get playRetry;

  /// No description provided for @openingStream.
  ///
  /// In zh, this message translates to:
  /// **'正在打开直播…'**
  String get openingStream;

  /// No description provided for @buffering.
  ///
  /// In zh, this message translates to:
  /// **'缓冲中…'**
  String get buffering;

  /// No description provided for @waitingForVideo.
  ///
  /// In zh, this message translates to:
  /// **'等待画面…'**
  String get waitingForVideo;

  /// No description provided for @tapToPlay.
  ///
  /// In zh, this message translates to:
  /// **'点按播放'**
  String get tapToPlay;

  /// No description provided for @actionFailed.
  ///
  /// In zh, this message translates to:
  /// **'{action} 失败：{error}'**
  String actionFailed(String action, String error);

  /// No description provided for @cohostInviteSent.
  ///
  /// In zh, this message translates to:
  /// **'连麦邀请已发送（{status}）'**
  String cohostInviteSent(String status);

  /// No description provided for @cohostAccepted.
  ///
  /// In zh, this message translates to:
  /// **'已接受连麦（{status}）'**
  String cohostAccepted(String status);

  /// No description provided for @cohostDeclined.
  ///
  /// In zh, this message translates to:
  /// **'已拒绝连麦（{status}）'**
  String cohostDeclined(String status);

  /// No description provided for @pkEndedWithWinner.
  ///
  /// In zh, this message translates to:
  /// **'PK 已结束 · 胜者 {winnerRoomId}'**
  String pkEndedWithWinner(String winnerRoomId);

  /// No description provided for @pkScoreLine.
  ///
  /// In zh, this message translates to:
  /// **'PK {status}：{scoreA} – {scoreB}'**
  String pkScoreLine(String status, int scoreA, int scoreB);

  /// No description provided for @pkScoreLineWinner.
  ///
  /// In zh, this message translates to:
  /// **'PK {status}：{scoreA} – {scoreB} · 胜 {winnerRoomId}'**
  String pkScoreLineWinner(
    String status,
    int scoreA,
    int scoreB,
    String winnerRoomId,
  );

  /// No description provided for @livekitJoinDetail.
  ///
  /// In zh, this message translates to:
  /// **'url: {url}\nroom: {room}\nidentity: {identity}\ntoken: {token}'**
  String livekitJoinDetail(
    String url,
    String room,
    String identity,
    String token,
  );

  /// No description provided for @labelCopied.
  ///
  /// In zh, this message translates to:
  /// **'已复制{label}'**
  String labelCopied(String label);

  /// No description provided for @coinsPriceLine.
  ///
  /// In zh, this message translates to:
  /// **'{coins} 金币 · {amount} {currency}'**
  String coinsPriceLine(int coins, String amount, String currency);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
