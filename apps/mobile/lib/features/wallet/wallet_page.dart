import 'dart:async';

import 'package:flutter/material.dart';

import '../../api/api_client.dart';
import '../../api/events_repository.dart';
import '../../api/gifts_repository.dart';
import '../../api/pay_repository.dart';
import '../../config/app_config.dart';

/// Wallet balance + mock pay packages (sandbox complete).
class WalletPage extends StatefulWidget {
  const WalletPage({
    super.key,
    required this.config,
    required this.accessToken,
    this.giftsRepository,
    this.payRepository,
    this.eventsRepository,
  });

  final AppConfig config;
  final String accessToken;
  final GiftsRepository? giftsRepository;
  final PayRepository? payRepository;
  final EventsRepository? eventsRepository;

  @override
  State<WalletPage> createState() => _WalletPageState();
}

class _WalletPageState extends State<WalletPage> {
  late final GiftsRepository _gifts;
  late final PayRepository _pay;
  late final EventsRepository _events;

  int _balance = 0;
  List<PayProduct> _products = [];
  List<LedgerEntry> _ledger = [];
  String? _ledgerHint;
  String? _error;
  String? _hint;
  bool _loading = true;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    final api = ApiClient(
      baseUrl: widget.config.normalizedApiBaseUrl,
      accessToken: widget.accessToken,
    );
    _gifts = widget.giftsRepository ?? GiftsRepository(client: api);
    _pay = widget.payRepository ?? PayRepository(client: api);
    _events = widget.eventsRepository ?? EventsRepository(client: api);
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
      _ledgerHint = null;
    });
    try {
      final balance = await _gifts.walletBalance();
      List<PayProduct> products = [];
      try {
        products = await _pay.listProducts();
      } catch (_) {
        // Pay catalog optional when channel disabled.
      }
      // Ledger is best-effort — wallet still usable if endpoint lags.
      final ledger = await _loadLedgerBestEffort();
      if (!mounted) return;
      setState(() {
        _balance = balance;
        _products = products;
        _ledger = ledger;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<List<LedgerEntry>> _loadLedgerBestEffort() async {
    try {
      final entries = await _gifts.listLedger();
      // Newest first when API returns chronological order.
      final sorted = List<LedgerEntry>.from(entries)
        ..sort((a, b) => b.createdAt.compareTo(a.createdAt));
      if (mounted) {
        setState(() => _ledgerHint = null);
      }
      return sorted.take(20).toList();
    } catch (_) {
      if (mounted) {
        setState(() => _ledgerHint = 'Ledger unavailable');
      }
      return const [];
    }
  }

  Future<void> _refreshLedgerQuiet() async {
    final ledger = await _loadLedgerBestEffort();
    if (!mounted) return;
    setState(() => _ledger = ledger);
  }

  Future<void> _mockTopup() async {
    setState(() => _busy = true);
    try {
      final balance = await _gifts.topup(100);
      if (!mounted) return;
      setState(() {
        _balance = balance;
        _hint = 'Mock topup +100';
        _busy = false;
      });
      unawaited(_refreshLedgerQuiet());
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  Future<void> _buy(PayProduct product) async {
    setState(() {
      _busy = true;
      _error = null;
      _hint = null;
    });
    try {
      final order = await _pay.createOrder(productId: product.id);
      unawaited(_events.track(
        'pay.order_create',
        props: {
          'product_id': product.id,
          'channel': order.channel,
          'order_id': order.id,
        },
      ));
      final done = await _pay.sandboxComplete(order.id);
      unawaited(_events.track(
        'pay.order_credit',
        props: {'order_id': done.id, 'channel': done.channel},
      ));
      final balance = await _gifts.walletBalance();
      if (!mounted) return;
      setState(() {
        _balance = balance;
        _hint =
            'Credited ${done.coins} coins (order ${done.id}, ${done.status})';
        _busy = false;
      });
      unawaited(_refreshLedgerQuiet());
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  String _formatLedgerLine(LedgerEntry e) {
    final sign = e.amount >= 0 ? '+' : '';
    final type = e.entryType.isEmpty ? 'entry' : e.entryType;
    final ref = e.reference.isEmpty ? '' : ' · ${e.reference}';
    return '$sign${e.amount} ($type)$ref → ${e.balanceAfter}';
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Wallet'),
        actions: [
          IconButton(onPressed: _loading ? null : _load, icon: const Icon(Icons.refresh)),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(24),
              children: [
                Text(
                  key: const Key('wallet-balance'),
                  'Balance: $_balance',
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 12),
                FilledButton.tonal(
                  key: const Key('wallet-mock-topup'),
                  onPressed: _busy ? null : _mockTopup,
                  child: const Text('Mock topup +100'),
                ),
                if (_hint != null) ...[
                  const SizedBox(height: 12),
                  Text(
                    key: const Key('wallet-hint'),
                    _hint!,
                    style: TextStyle(color: Theme.of(context).colorScheme.primary),
                  ),
                ],
                if (_error != null) ...[
                  const SizedBox(height: 12),
                  Text(
                    _error!,
                    style: TextStyle(color: Theme.of(context).colorScheme.error),
                  ),
                ],
                const SizedBox(height: 24),
                Text(
                  'Recent ledger',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                if (_ledger.isEmpty)
                  Text(
                    key: const Key('wallet-ledger-empty'),
                    _ledgerHint ?? 'No ledger entries yet',
                    style: TextStyle(color: Theme.of(context).colorScheme.onSurfaceVariant),
                  )
                else
                  ..._ledger.map(
                    (e) => ListTile(
                      key: Key('wallet-ledger-${e.id}'),
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(_formatLedgerLine(e)),
                      subtitle: e.createdAt.isEmpty ? null : Text(e.createdAt),
                    ),
                  ),
                const SizedBox(height: 24),
                Text(
                  'Coin packages',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                if (_products.isEmpty)
                  const Text('No pay products (enable PAY_CHANNELS=mock).')
                else
                  ..._products.map(
                    (p) => Card(
                      key: Key('pay-product-${p.id}'),
                      child: ListTile(
                        title: Text(p.title),
                        subtitle: Text('${p.coins} coins · ${p.amount} ${p.currency}'),
                        trailing: FilledButton(
                          onPressed: _busy ? null : () => _buy(p),
                          child: const Text('Buy (sandbox)'),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
    );
  }
}
