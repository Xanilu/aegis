import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:crypto/crypto.dart' as c;

void main() => runApp(const AegisApp());

class AegisApp extends StatelessWidget {
  const AegisApp({super.key});
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Aegis Relay Tester',
      debugShowCheckedModeBanner: false,
      home: const RelayHome(),
    );
  }
}

class RelayHome extends StatefulWidget {
  const RelayHome({super.key});
  @override
  State<RelayHome> createState() => _RelayHomeState();
}

class _RelayHomeState extends State<RelayHome> {
  // Hinweis:
  // - Desktop/iOS: http://127.0.0.1:3000
  // - Android-Emulator: http://10.0.2.2:3000
  final relayCtrl = TextEditingController(text: 'http://127.0.0.1:3000');
  final toIdCtrl = TextEditingController(text: 'alice');
  final powPrefixCtrl = TextEditingController(text: '0000'); // muss zum Server passen
  String log = '';

  void append(String s) {
    setState(() => log = '${DateTime.now().toIso8601String()}  $s\n$log');
  }

  /// erzeugt exakt 4096 zufällige Bytes und gibt Base64 zurück
  String randomBlobBase64() {
    final rnd = Random.secure();
    final bytes = Uint8List(4096);
    for (int i = 0; i < bytes.length; i++) {
      bytes[i] = rnd.nextInt(256);
    }
    return base64Encode(bytes);
  }

  /// PoW: finde kleinste hex-Nonce, so dass sha256(nonce + to_id) mit prefix beginnt
  String findNonce(String toId, String prefix, {int maxIters = 1 << 24}) {
    if (prefix.isEmpty) return '';
    int i = 0;
    while (i < maxIters) {
      final nonce = i.toRadixString(16);
      final bytes = utf8.encode(nonce + toId);
      final digest = c.sha256.convert(bytes);
      final hex = digest.toString(); // already lowercase hex
      if (hex.startsWith(prefix)) return nonce;
      i++;
    }
    throw Exception('PoW not found within $maxIters iterations (prefix=$prefix)');
  }

  Future<void> sendBlob() async {
    final base = relayCtrl.text.trim();
    final toId = toIdCtrl.text.trim();
    final prefix = powPrefixCtrl.text.trim();
    if (base.isEmpty || toId.isEmpty) {
      append('WARN: Relay URL oder to_id leer');
      return;
    }
    final url = Uri.parse('$base/v1/envelopes');
    final blob = randomBlobBase64();

    String nonce = '';
    try {
      nonce = findNonce(toId, prefix);
    } catch (e) {
      append('ERROR PoW: $e');
      return;
    }

    try {
      final res = await http.put(
        url,
        headers: {
          'Content-Type': 'application/json',
          if (nonce.isNotEmpty) 'X-POW-Nonce': nonce,
        },
        body: jsonEncode({'to_id': toId, 'cipher_blob': blob}),
      );
      append('SEND ${res.statusCode}: ${res.body}');
    } catch (e) {
      append('ERROR send: $e');
    }
  }

  Future<void> fetchMailbox() async {
    final base = relayCtrl.text.trim();
    final toId = toIdCtrl.text.trim();
    if (base.isEmpty || toId.isEmpty) {
      append('WARN: Relay URL oder to_id leer');
      return;
    }
    final url = Uri.parse('$base/v1/mailbox?for=$toId');
    try {
      final res = await http.get(url);
      append('FETCH ${res.statusCode}: ${res.body}');
    } catch (e) {
      append('ERROR fetch: $e');
    }
  }

  Future<void> ping() async {
    final base = relayCtrl.text.trim();
    try {
      final res = await http.get(Uri.parse('$base/ping'));
      append('PING ${res.statusCode}: ${res.body}');
    } catch (e) {
      append('ERROR ping: $e');
    }
  }

  Future<void> health() async {
    final base = relayCtrl.text.trim();
    try {
      final res = await http.get(Uri.parse('$base/health'));
      append('HEALTH ${res.statusCode}: ${res.body}');
    } catch (e) {
      append('ERROR health: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final pad = const EdgeInsets.all(16);
    return Scaffold(
      appBar: AppBar(title: const Text('Aegis Relay Tester')),
      body: Padding(
        padding: pad,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: relayCtrl,
              decoration: const InputDecoration(
                labelText: 'Relay URL',
                hintText: 'http://127.0.0.1:3000 (Desktop) / http://10.0.2.2:3000 (Android)',
              ),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: toIdCtrl,
                    decoration: const InputDecoration(
                      labelText: 'to_id / mailbox',
                      hintText: 'alice',
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                SizedBox(
                  width: 120,
                  child: TextField(
                    controller: powPrefixCtrl,
                    decoration: const InputDecoration(
                      labelText: 'PoW Prefix',
                      hintText: '0000',
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                ElevatedButton(onPressed: ping, child: const Text('PING')),
                ElevatedButton(onPressed: health, child: const Text('HEALTH')),
                ElevatedButton(onPressed: sendBlob, child: const Text('4096B SENDEN')),
                ElevatedButton(onPressed: fetchMailbox, child: const Text('MAILBOX HOLEN')),
              ],
            ),
            const SizedBox(height: 16),
            const Text('Log:'),
            const SizedBox(height: 8),
            Expanded(
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.black12),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SingleChildScrollView(
                  reverse: true,
                  child: Text(
                    log,
                    style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
