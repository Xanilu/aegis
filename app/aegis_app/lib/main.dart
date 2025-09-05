import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:crypto/crypto.dart' as c;
import 'package:cryptography/cryptography.dart' as cg;
import 'package:qr_flutter/qr_flutter.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

void main() => runApp(const AegisApp());

class AegisApp extends StatelessWidget {
  const AegisApp({super.key});
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Aegis Relay Tester (E2E v0)',
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
  // --- UI State ---
  final relayCtrl = TextEditingController(text: 'http://127.0.0.1:3000'); // Android-Emu: http://10.0.2.2:3000
  final toIdCtrl = TextEditingController(text: 'alice');
  final powPrefixCtrl = TextEditingController(text: '0000'); // muss zum Server passen
  final messageCtrl = TextEditingController(text: 'Hallo von Aegis 👋');

  String log = '';
  void append(String s) => setState(() => log = '${DateTime.now().toIso8601String()}  $s\n$log');

  // --- Crypto State ---
  cg.SimpleKeyPair? _localKeyPair;
  String? _localPubB64;
  final peerPubCtrl = TextEditingController();

  static const int frameBytes = 4096;
  static const int macLen = 16; // AES-GCM Tag
  static const int algoAesGcm = 1;
  static const String magic = 'AG'; // "Aegis"

  final _rnd = Random.secure();
  final _aesGcm = cg.AesGcm.with256bits();

  // ---------- Key mgmt ----------
  Future<void> ensureLocalKey() async {
    if (_localKeyPair != null) return;
    final x = cg.X25519();
    _localKeyPair = await x.newKeyPair();
    final pub = await _localKeyPair!.extractPublicKey();
    _localPubB64 = base64Encode(pub.bytes);
    append('Keygen ok. PublicKey bereit.');
    setState(() {});
  }

  Future<cg.SecretKey> deriveShared() async {
    await ensureLocalKey();
    final peerB64 = peerPubCtrl.text.trim();
    if (peerB64.isEmpty) throw Exception('Kein Kontakt-Public-Key gesetzt.');
    final peerBytes = base64Decode(peerB64);
    final x = cg.X25519();
    final secret = await x.sharedSecretKey(
      keyPair: _localKeyPair!,
      remotePublicKey: cg.SimplePublicKey(peerBytes, type: cg.KeyPairType.x25519),
    );
    final raw = await secret.extractBytes();
    final keyBytes = c.sha256.convert(raw).bytes; // MVP: SHA-256(abgeleitetes Secret)
    return cg.SecretKey(keyBytes);
  }

  Uint8List _random(int n) {
    final b = Uint8List(n);
    for (var i = 0; i < n; i++) b[i] = _rnd.nextInt(256);
    return b;
  }

  // ---------- Encrypt/Decrypt (Frame = 4096B) ----------
  // Header V2: magic(2) ver(1) algo(1) nonce(12) pt_len(2) => 18 bytes
  Future<String> encryptToFrameBase64V2(String plaintext) async {
    final key = await deriveShared();
    final nonce = _random(12);
    final pt = utf8.encode(plaintext);
    if (pt.length > frameBytes - (18 + macLen)) {
      throw Exception('Plaintext zu groß für Frame');
    }
    final box = await _aesGcm.encrypt(pt, secretKey: key, nonce: nonce);
    final cipher = Uint8List.fromList(box.cipherText);
    final tag = Uint8List.fromList(box.mac.bytes);

    final header = BytesBuilder();
    header.add(utf8.encode(magic));        // 0..1
    header.add([1]);                       // 2: version
    header.add([algoAesGcm]);              // 3: algo
    header.add(nonce);                     // 4..15 (12)
    final ptLen = Uint8List(2)..buffer.asByteData().setUint16(0, pt.length, Endian.big);
    header.add(ptLen);                     // 16..17
    final head = header.toBytes();         // 18 bytes

    final body = BytesBuilder();
    body.add(head);
    body.add(cipher);
    body.add(tag);
    var framed = body.toBytes();

    if (framed.length > frameBytes) throw Exception('Nachricht zu groß');
    if (framed.length < frameBytes) {
      final pad = _random(frameBytes - framed.length);
      final out = BytesBuilder();
      out.add(framed);
      out.add(pad);
      framed = out.toBytes();
    }
    return base64Encode(framed);
  }

  Future<String> decryptFromFrameBase64V2(String b64) async {
    final key = await deriveShared();
    final bytes = base64Decode(b64);
    if (bytes.length != frameBytes) throw Exception('Frame != 4096B');
    if (bytes[0] != 0x41 || bytes[1] != 0x47) throw Exception('MAGIC mismatch'); // 'A','G'
    final ver = bytes[2];
    final algo = bytes[3];
    if (ver != 1 || algo != algoAesGcm) throw Exception('Version/Algo nicht unterstützt');

    final nonce = bytes.sublist(4, 16); // 12 bytes
    final ptLen = bytes.buffer.asByteData().getUint16(16, Endian.big);
    final off = 18;

    final cipher = bytes.sublist(off, off + ptLen);
    final mac = bytes.sublist(off + ptLen, off + ptLen + macLen);

    final secretBox = cg.SecretBox(cipher, nonce: nonce, mac: cg.Mac(mac));
    final pt = await _aesGcm.decrypt(secretBox, secretKey: key);
    return utf8.decode(pt);
  }

  // ---------- Networking & PoW ----------
  String _findNonce(String toId, String prefix, {int maxIters = 1 << 24}) {
    if (prefix.isEmpty) return '';
    int i = 0;
    while (i < maxIters) {
      final nonce = i.toRadixString(16);
      final hex = c.sha256.convert(utf8.encode(nonce + toId)).toString();
      if (hex.startsWith(prefix)) return nonce;
      i++;
    }
    throw Exception('PoW not found');
  }

  Future<void> sendText() async {
    try {
      await ensureLocalKey();
      final base = relayCtrl.text.trim();
      final toId = toIdCtrl.text.trim();
      final prefix = powPrefixCtrl.text.trim();
      final url = Uri.parse('$base/v1/envelopes');
      final frameB64 = await encryptToFrameBase64V2(messageCtrl.text);
      final nonce = _findNonce(toId, prefix);
      final res = await http.put(
        url,
        headers: {
          'Content-Type': 'application/json',
          if (nonce.isNotEmpty) 'X-POW-Nonce': nonce,
        },
        body: jsonEncode({'to_id': toId, 'cipher_blob': frameB64}),
      );
      append('SEND ${res.statusCode}: ${res.body}');
    } catch (e) {
      append('ERROR sendText: $e');
    }
  }

  Future<void> fetchAndDecrypt() async {
    try {
      await ensureLocalKey();
      final base = relayCtrl.text.trim();
      final toId = toIdCtrl.text.trim();
      final url = Uri.parse('$base/v1/mailbox?for=$toId');
      final res = await http.get(url);
      append('FETCH ${res.statusCode}');
      if (res.statusCode != 200) {
        append('Body: ${res.body}');
        return;
      }
      final json = jsonDecode(res.body) as Map<String, dynamic>;
      final envs = (json['envelopes'] as List).cast<Map<String, dynamic>>();
      if (envs.isEmpty) {
        append('Leere Mailbox.');
        return;
      }
      for (final e in envs) {
        final b64 = e['cipher_blob'] as String;
        final text = await decryptFromFrameBase64V2(b64);
        append('📩 $text');
      }
    } catch (e) {
      append('ERROR fetchAndDecrypt: $e');
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

  // ---------- UI ----------
  @override
  Widget build(BuildContext context) {
    final pad = const EdgeInsets.all(16);
    return Scaffold(
      appBar: AppBar(title: const Text('Aegis Relay Tester (E2E v0)')),
      body: Padding(
        padding: pad,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: relayCtrl,
              decoration: const InputDecoration(
                labelText: 'Relay URL',
                hintText: 'Desktop: http://127.0.0.1:3000 • Android-Emu: http://10.0.2.2:3000',
              ),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: toIdCtrl,
                    decoration: const InputDecoration(labelText: 'to_id / mailbox'),
                  ),
                ),
                const SizedBox(width: 12),
                SizedBox(
                  width: 110,
                  child: TextField(
                    controller: powPrefixCtrl,
                    decoration: const InputDecoration(labelText: 'PoW Prefix'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: TextEditingController(text: _localPubB64 ?? ''),
                    readOnly: true,
                    decoration: const InputDecoration(labelText: 'Mein Public Key (b64)'),
                  ),
                ),
                const SizedBox(width: 12),
                ElevatedButton(
                  onPressed: () async => await ensureLocalKey(),
                  child: const Text('Keygen'),
                ),
              ],
            ),
            const SizedBox(height: 8),

            if (_localPubB64 != null && _localPubB64!.isNotEmpty) ...[
              const Text('Mein Public Key (QR):'),
              const SizedBox(height: 8),
              Center(
                child: QrImageView(
                  data: 'aegis:pk1:${_localPubB64!}',
                  size: 160,
                  gapless: true,
                ),
              ),
              const SizedBox(height: 12),
            ],

            TextField(
              controller: peerPubCtrl,
              decoration: const InputDecoration(
                labelText: 'Kontakt Public Key (b64)',
                hintText: 'Per QR scannen oder Base64 einfügen',
              ),
            ),

            const SizedBox(height: 8),
            Row(
              children: [
                ElevatedButton(
                  onPressed: () async {
                    final res = await Navigator.of(context).push<String>(
                      MaterialPageRoute(builder: (_) => const ScanPage()),
                    );
                    if (res != null && res.isNotEmpty) {
                      final b64 = res.startsWith('aegis:pk1:') ? res.substring('aegis:pk1:'.length) : res;
                      try {
                        final raw = base64Decode(b64);
                        if (raw.length != 32) throw Exception('Kein X25519 Public Key (32B erwartet)');
                        peerPubCtrl.text = b64;
                        append('Kontakt-Public-Key übernommen (QR).');
                        setState(() {});
                      } catch (e) {
                        append('QR ungültig: $e');
                      }
                    }
                  },
                  child: const Text('QR scannen'),
                ),
                const SizedBox(width: 12),
                ElevatedButton(
                  onPressed: () async {
                    await showDialog(
                      context: context,
                      builder: (_) => AlertDialog(
                        title: const Text('Mein Public Key (QR)'),
                        content: _localPubB64 == null
                            ? const Text('Bitte erst Keygen ausführen.')
                            : QrImageView(data: 'aegis:pk1:${_localPubB64!}', size: 240),
                      ),
                    );
                  },
                  child: const Text('Meinen QR anzeigen'),
                ),
              ],
            ),

            const SizedBox(height: 12),
            TextField(
              controller: messageCtrl,
              decoration: const InputDecoration(labelText: 'Nachricht (Text)'),
            ),

            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              children: [
                ElevatedButton(onPressed: ping, child: const Text('PING')),
                ElevatedButton(onPressed: health, child: const Text('HEALTH')),
                ElevatedButton(onPressed: sendText, child: const Text('TEXT SENDEN (E2E)')),
                ElevatedButton(onPressed: fetchAndDecrypt, child: const Text('HOLEN & ENTCRYPTEN')),
              ],
            ),
            const SizedBox(height: 12),
            const Text('Log:'),
            const SizedBox(height: 6),
            Expanded(
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.black12),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SingleChildScrollView(
                  reverse: true,
                  child: Text(log, style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ---------- QR Scan Page ----------
class ScanPage extends StatefulWidget {
  const ScanPage({super.key});
  @override
  State<ScanPage> createState() => _ScanPageState();
}
class _ScanPageState extends State<ScanPage> {
  bool _handled = false;
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Kontakt-QR scannen')),
      body: MobileScanner(
        onDetect: (capture) {
          if (_handled) return;
          final codes = capture.barcodes;
          for (final bc in codes) {
            final raw = bc.rawValue ?? '';
            if (raw.isEmpty) continue;
            _handled = true;
            Navigator.of(context).pop(raw);
            return;
          }
        },
      ),
    );
  }
}
