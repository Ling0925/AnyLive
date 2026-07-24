import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/events_repository.dart';
import 'package:anylive_mobile/api/gifts_repository.dart';
import 'package:anylive_mobile/api/pay_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/wallet/wallet_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:anylive_mobile/l10n/l10n.dart';

ApiClient _dummy() => ApiClient(baseUrl: 'http://x', accessToken: 't');

class _FakeGifts extends GiftsRepository {
  _FakeGifts() : super(client: _dummy());
  int balance = 42;

  @override
  Future<int> walletBalance() async => balance;

  @override
  Future<int> topup(int amount) async {
    balance += amount;
    return balance;
  }
}

class _FakePay extends PayRepository {
  _FakePay() : super(client: _dummy());

  @override
  Future<List<PayProduct>> listProducts() async => [
        PayProduct(
          id: 'p1',
          sku: 'coins_100',
          title: '100 coins',
          coins: 100,
          amount: '0.99',
          currency: 'USD',
        ),
      ];

  @override
  Future<PayOrder> createOrder({
    required String productId,
    String channel = 'mock',
    String? clientRequestId,
  }) async =>
      PayOrder(
        id: 'ord1',
        status: 'pending',
        coins: 100,
        amount: '0.99',
        currency: 'USD',
        channel: channel,
      );

  @override
  Future<PayOrder> sandboxComplete(String orderId) async => PayOrder(
        id: orderId,
        status: 'credited',
        coins: 100,
        amount: '0.99',
        currency: 'USD',
        channel: 'mock',
      );
}

class _FakeEvents extends EventsRepository {
  _FakeEvents() : super(client: _dummy());
  final List<String> tracked = [];

  @override
  Future<void> track(
    String name, {
    Map<String, dynamic>? props,
    String? clientEventId,
  }) async {
    tracked.add(name);
  }
}

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://x',
    environment: 'dev',
  );

  testWidgets('wallet shows balance and sandbox buy credits', (tester) async {
    final gifts = _FakeGifts();
    final pay = _FakePay();
    final events = _FakeEvents();
    await tester.pumpWidget(
      MaterialApp(
        
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: WalletPage(
          config: config,
          accessToken: 't',
          giftsRepository: gifts,
          payRepository: pay,
          eventsRepository: events,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('wallet-balance')), findsOneWidget);
    expect(find.text('Balance: 42'), findsOneWidget);
    expect(find.byKey(const Key('pay-product-p1')), findsOneWidget);

    await tester.tap(find.text('Buy (sandbox)'));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('wallet-hint')), findsOneWidget);
    expect(events.tracked, contains('pay.order_create'));
    expect(events.tracked, contains('pay.order_credit'));
  });

  testWidgets('mock topup updates balance', (tester) async {
    final gifts = _FakeGifts();
    await tester.pumpWidget(
      MaterialApp(
        
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: WalletPage(
          config: config,
          accessToken: 't',
          giftsRepository: gifts,
          payRepository: _FakePay(),
          eventsRepository: _FakeEvents(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('wallet-mock-topup')));
    await tester.pumpAndSettle();
    expect(find.text('Balance: 142'), findsOneWidget);
    expect(find.byKey(const Key('wallet-hint')), findsOneWidget);
  });
}
