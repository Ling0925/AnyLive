// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'AnyLive';

  @override
  String appTitleFlavor(String flavor) {
    return 'AnyLive ($flavor)';
  }

  @override
  String get appTagline => 'AnyLive Mobile';

  @override
  String envFlavorLine(String env, String flavor) {
    return 'env: $env · $flavor';
  }

  @override
  String envLine(String env) {
    return 'env: $env';
  }

  @override
  String flavorLine(String flavor) {
    return 'flavor: $flavor';
  }

  @override
  String apiLine(String url) {
    return 'api: $url';
  }

  @override
  String brandFlavor(String flavor) {
    return 'AnyLive · $flavor';
  }

  @override
  String get navHome => 'Home';

  @override
  String get navFollowing => 'Following';

  @override
  String get navGoLive => 'Go Live';

  @override
  String get goLiveAction => 'Go live';

  @override
  String get navYou => 'You';

  @override
  String get signIn => 'Sign in';

  @override
  String get language => 'Language';

  @override
  String get languageSystem => 'System';

  @override
  String get languageChinese => '中文';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageSection => 'Language & display';

  @override
  String get loginTitle => 'AnyLive Login';

  @override
  String get loginHeadline => 'Sign in with email';

  @override
  String get loginDevOtpHint =>
      'Local dogfood: fixed OTP 123456 when ALLOW_DEV_OTP=1.';

  @override
  String get loginOtpHint =>
      'We email a one-time code. Check spam if it is slow.';

  @override
  String get email => 'Email';

  @override
  String get otpCode => 'OTP code';

  @override
  String get otpCodeDev => 'OTP (dev: 123456)';

  @override
  String get sendOtp => 'Send OTP';

  @override
  String get resendCode => 'Resend code';

  @override
  String resendInSeconds(int seconds) {
    return 'Resend in ${seconds}s';
  }

  @override
  String get ageConfirm => 'I confirm I am 18 or older';

  @override
  String get privacyAccept => 'I accept the privacy policy';

  @override
  String get pleaseWait => 'Please wait…';

  @override
  String get verifyContinue => 'Verify & continue';

  @override
  String get useDifferentEmail => 'Use a different email';

  @override
  String get privacyPolicy => 'Privacy Policy';

  @override
  String get termsOfService => 'Terms of Service';

  @override
  String get enterValidEmail => 'Enter a valid email address.';

  @override
  String get enterOtpFromEmail => 'Enter the OTP code from your email.';

  @override
  String get errInvalidRequest => 'Invalid request. Check email and code.';

  @override
  String get errInvalidOrExpiredOtp =>
      'Invalid or expired code. Request a new OTP.';

  @override
  String get errOtpNotFound => 'Code not found. Send OTP first.';

  @override
  String get errTooManyAttempts =>
      'Too many attempts. Wait a moment and try again.';

  @override
  String errCannotReachApi(String url) {
    return 'Cannot reach API ($url).';
  }

  @override
  String get youTitle => 'You';

  @override
  String signedInAs(String name) {
    return 'signed in as $name';
  }

  @override
  String get sectionAccount => 'Account';

  @override
  String get sectionSession => 'Session';

  @override
  String get sectionAbout => 'About';

  @override
  String get profilePrivacy => 'Profile & privacy';

  @override
  String get profilePrivacySub => 'Edit name, privacy, export / delete';

  @override
  String get wallet => 'Wallet';

  @override
  String get walletSub => 'Balance, top-up, ledger';

  @override
  String get logout => 'Logout';

  @override
  String get logoutSub => 'Revoke this device session';

  @override
  String get logoutConfirmTitle => 'Log out?';

  @override
  String get logoutConfirmBody =>
      'You will need to sign in again with email OTP.';

  @override
  String get cancel => 'Cancel';

  @override
  String get logOut => 'Log out';

  @override
  String get editProfile => 'Edit profile';

  @override
  String get displayName => 'Display name';

  @override
  String get displayNameRequired => 'Display name is required';

  @override
  String get avatarUrl => 'Avatar URL';

  @override
  String get regionHint => 'Region (e.g. US, SG)';

  @override
  String get save => 'Save';

  @override
  String get saving => 'Saving…';

  @override
  String get profileSaved => 'Profile saved';

  @override
  String get avatarUrlSet => 'Avatar URL set';

  @override
  String get creatorCenter => 'Creator center';

  @override
  String get loadingStats => 'Loading stats…';

  @override
  String get followers => 'Followers';

  @override
  String get following => 'Following';

  @override
  String get liveRooms => 'Live rooms';

  @override
  String get totalRooms => 'Total rooms';

  @override
  String get giftCoins => 'Gift coins';

  @override
  String get giftCredits => 'Gift credits';

  @override
  String get yourRooms => 'Your rooms';

  @override
  String get privacyData => 'Privacy & data';

  @override
  String get exportMyData => 'Export my data';

  @override
  String exportCopied(int chars) {
    return 'Export copied ($chars chars)';
  }

  @override
  String get signOutAllDevices => 'Sign out all devices';

  @override
  String signOutAllDevicesCount(int count) {
    return 'Sign out all devices ($count)';
  }

  @override
  String get signOutAllConfirmTitle => 'Sign out all devices?';

  @override
  String get signOutAllConfirmBody =>
      'Revokes every refresh session. This device will need to log in again.';

  @override
  String get signOutAll => 'Sign out all';

  @override
  String revokedSessions(int count) {
    return 'Revoked $count session(s)';
  }

  @override
  String get sessionRevoked => 'Session revoked';

  @override
  String get activeSessions => 'Active sessions';

  @override
  String get activeSessionsHint =>
      'Each row is a refresh token. Revoking ends that device.';

  @override
  String get revoke => 'Revoke';

  @override
  String get deleteAccount => 'Delete account';

  @override
  String get deleteAccountConfirmTitle => 'Delete account?';

  @override
  String get deleteAccountConfirmBody =>
      'This soft-deletes your account. You will be signed out.';

  @override
  String get delete => 'Delete';

  @override
  String get close => 'Close';

  @override
  String get retry => 'Retry';

  @override
  String get copy => 'Copy';

  @override
  String get submit => 'Submit';

  @override
  String get start => 'Start';

  @override
  String get invite => 'Invite';

  @override
  String get more => 'More';

  @override
  String get reason => 'Reason';

  @override
  String get refresh => 'Refresh';

  @override
  String balance(int amount) {
    return 'Balance: $amount';
  }

  @override
  String get recentLedger => 'Recent ledger';

  @override
  String get noLedgerEntries => 'No ledger entries yet';

  @override
  String get ledgerUnavailable => 'Ledger unavailable';

  @override
  String get coinPackages => 'Coin packages';

  @override
  String get noPayProducts => 'No pay products (enable PAY_CHANNELS=mock).';

  @override
  String get buySandbox => 'Buy (sandbox)';

  @override
  String get mockTopup => 'Mock topup +100';

  @override
  String get toppedUp => 'Topped up +100';

  @override
  String creditedCoins(int coins, String orderId, String status) {
    return 'Credited $coins coins (order $orderId, $status)';
  }

  @override
  String get topUp => 'Top up';

  @override
  String get feedHome => 'Home';

  @override
  String get feedFollowing => 'Following';

  @override
  String get feedDiscover => 'Discover';

  @override
  String get feedHot => 'Hot';

  @override
  String get searchRoomsOrUsers => 'Search rooms or users';

  @override
  String get noHotRooms => 'No hot rooms';

  @override
  String get noFollowingRooms => 'No rooms from people you follow';

  @override
  String get browseHome => 'Browse Home';

  @override
  String roomStatusLine(String status) {
    return 'room · $status';
  }

  @override
  String userIdLine(String id) {
    return 'user · $id';
  }

  @override
  String get goLiveTitle => 'Go Live';

  @override
  String get youAreLive => 'You are live';

  @override
  String get broadcastWithObs => 'Broadcast with OBS';

  @override
  String get roomTitle => 'Room title';

  @override
  String get working => 'Working…';

  @override
  String get startLive => 'Start live';

  @override
  String get endLive => 'End live';

  @override
  String get stopLive => 'Stop live';

  @override
  String get openMyRoom => 'Open my room';

  @override
  String get openRoom => 'Open room';

  @override
  String roomStatus(String status) {
    return 'status: $status';
  }

  @override
  String get obsServer => 'OBS Server';

  @override
  String get obsServerCustom => 'OBS Server (Custom RTMP)';

  @override
  String get obsStreamKey => 'OBS Stream key';

  @override
  String get server => 'Server';

  @override
  String get streamKey => 'Stream key';

  @override
  String get refreshObsKeys => 'Refresh OBS keys';

  @override
  String get copyServer => 'Copy server';

  @override
  String get copyKey => 'Copy key';

  @override
  String get copyStreamKey => 'Copy stream key';

  @override
  String get obsInstructions => 'In OBS: Settings → Stream → Service = Custom.';

  @override
  String get obsKeySeparate =>
      'Paste Server and Stream key separately — do not put the key in the server URL.';

  @override
  String get liveEnded => 'Live ended';

  @override
  String get starting => 'Starting…';

  @override
  String get noLiveRooms => 'No live rooms';

  @override
  String roomIdLine(String id) {
    return 'Room: $id';
  }

  @override
  String get unavailable => '(unavailable)';

  @override
  String get defaultLiveTitle => 'My Live';

  @override
  String get untitledLive => 'Untitled live';

  @override
  String get liveBadge => 'LIVE';

  @override
  String get shareLive => 'Share live';

  @override
  String get linkCopied => 'Link copied';

  @override
  String get copyLink => 'Copy link';

  @override
  String get unfollowedHost => 'Unfollowed host';

  @override
  String get followingHost => 'Following host';

  @override
  String get reportSubmitted => 'Report submitted';

  @override
  String get reportRoom => 'Report room';

  @override
  String get follow => 'Follow';

  @override
  String get unfollow => 'Unfollow';

  @override
  String likeCount(int count) {
    return 'Like ($count)';
  }

  @override
  String get share => 'Share';

  @override
  String get report => 'Report';

  @override
  String get host => 'Host';

  @override
  String get noMessagesYet => 'No messages yet';

  @override
  String get saySomething => 'Say something…';

  @override
  String get roomOfflineChatDisabled => 'Room offline — chat send disabled';

  @override
  String get streamEndedChatClosed => 'Stream ended — chat closed';

  @override
  String get recordingEnabled => 'Recording enabled';

  @override
  String get recordingDisabled => 'Recording disabled';

  @override
  String get enableRecording => 'Enable recording';

  @override
  String get disableRecording => 'Disable recording';

  @override
  String get obsPublish => 'OBS publish';

  @override
  String get liveStartedObsHint =>
      'Live started — use OBS publish for stream key';

  @override
  String get liveStoppedIdle => 'Live stopped — room idle (not closed)';

  @override
  String get inviteCohost => 'Invite co-host';

  @override
  String get acceptCohost => 'Accept co-host';

  @override
  String get declineCohost => 'Decline co-host';

  @override
  String get inviteeUserId => 'Invitee user id (UUID)';

  @override
  String get startPk => 'Start PK';

  @override
  String get endPk => 'End PK';

  @override
  String get opponentRoomId => 'Opponent room id (UUID)';

  @override
  String get pkStarted => 'PK started';

  @override
  String get pkEnded => 'PK ended';

  @override
  String get pkUnavailable => 'PK unavailable (feature off)';

  @override
  String get cohostUnavailable => 'co-host unavailable (feature off)';

  @override
  String get inviteUnavailable => 'invite unavailable (feature off)';

  @override
  String get livekitJoin => 'LiveKit join';

  @override
  String get copyToken => 'Copy token';

  @override
  String get statusEnded => 'ENDED';

  @override
  String get streamEnded => 'Stream ended';

  @override
  String get hostOffline => 'Host offline';

  @override
  String get livePlayUrlUnavailable => 'Live — play URL unavailable';

  @override
  String get openStreamExternal => 'Open stream URL in external player';

  @override
  String get roomForceClosed => 'This room was force-closed';

  @override
  String get hostStoppedMayReturn => 'Host stopped — may go live again';

  @override
  String get copiedStreamUrl => 'Copied stream URL';

  @override
  String get hlsBrowserHlsJs => 'HLS (browser · hls.js · muted)';

  @override
  String get hlsBrowserMuted => 'HLS (browser · muted autoplay)';

  @override
  String get hlsInApp => 'HLS (in-app)';

  @override
  String get hlsStream => 'HLS stream';

  @override
  String get playerDisabledCopyUrl =>
      'In-app player disabled — copy URL to open externally';

  @override
  String get browserAutoplayMuted =>
      'Browser autoplay is muted. Use player controls to unmute.';

  @override
  String get copyStreamUrl => 'Copy stream URL';

  @override
  String get playRetry => 'Play / retry';

  @override
  String get openingStream => 'Opening stream…';

  @override
  String get buffering => 'Buffering…';

  @override
  String get waitingForVideo => 'Waiting for video…';

  @override
  String get tapToPlay => 'Tap to play';

  @override
  String actionFailed(String action, String error) {
    return '$action failed: $error';
  }

  @override
  String cohostInviteSent(String status) {
    return 'Co-host invite sent ($status)';
  }

  @override
  String cohostAccepted(String status) {
    return 'Co-host accepted ($status)';
  }

  @override
  String cohostDeclined(String status) {
    return 'Co-host declined ($status)';
  }

  @override
  String pkEndedWithWinner(String winnerRoomId) {
    return 'PK ended · winner $winnerRoomId';
  }

  @override
  String pkScoreLine(String status, int scoreA, int scoreB) {
    return 'PK $status: $scoreA – $scoreB';
  }

  @override
  String pkScoreLineWinner(
    String status,
    int scoreA,
    int scoreB,
    String winnerRoomId,
  ) {
    return 'PK $status: $scoreA – $scoreB · win $winnerRoomId';
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
    return '$label copied';
  }

  @override
  String coinsPriceLine(int coins, String amount, String currency) {
    return '$coins coins · $amount $currency';
  }
}
