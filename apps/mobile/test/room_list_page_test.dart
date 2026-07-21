import 'package:anylive_mobile/api/api_client.dart';
import 'package:anylive_mobile/api/rooms_repository.dart';
import 'package:anylive_mobile/config/app_config.dart';
import 'package:anylive_mobile/features/rooms/room_list_page.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class FakeRoomsRepo extends RoomsRepository {
  FakeRoomsRepo({
    this.live = const [],
    this.created,
    this.started,
    this.publish,
  }) : super(client: ApiClient(baseUrl: 'http://test'));

  List<Room> live;
  Room? created;
  Room? started;
  PublishInfo? publish;
  int listCalls = 0;
  int createCalls = 0;
  int startCalls = 0;
  int publishCalls = 0;

  @override
  Future<List<Room>> listRooms({String? status}) async {
    listCalls++;
    return live;
  }

  @override
  Future<Room> createRoom(String title) async {
    createCalls++;
    return created ??
        Room(id: 'r-new', ownerId: 'me', title: title, status: 'idle');
  }

  @override
  Future<Room> startRoom(String roomId) async {
    startCalls++;
    final room = started ??
        Room(id: roomId, ownerId: 'me', title: 'Live', status: 'live');
    live = [room];
    return room;
  }

  @override
  Future<PublishInfo> publishInfo(String roomId) async {
    publishCalls++;
    return publish ??
        PublishInfo(
          pushUrl: 'rtmp://localhost:1935/live/$roomId',
          streamKey: roomId,
        );
  }
}

void main() {
  const config = AppConfig(
    apiBaseUrl: 'http://localhost:8088',
    environment: 'local',
  );

  testWidgets('room list page shows title', (tester) async {
    final fake = FakeRoomsRepo();
    await tester.pumpWidget(
      MaterialApp(
        home: RoomListPage(
          config: config,
          accessToken: 'tok',
          roomsRepository: fake,
        ),
      ),
    );
    expect(find.text('Live rooms'), findsOneWidget);
    await tester.pumpAndSettle();
    expect(find.text('No live rooms'), findsOneWidget);
  });

  testWidgets('go live shows OBS publish dialog', (tester) async {
    final fake = FakeRoomsRepo(
      created: Room(
        id: 'r-host',
        ownerId: 'me',
        title: 'My Live',
        status: 'idle',
      ),
      started: Room(
        id: 'r-host',
        ownerId: 'me',
        title: 'My Live',
        status: 'live',
      ),
      publish: PublishInfo(
        pushUrl: 'rtmp://localhost:1935/live/r-host',
        streamKey: 'r-host',
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: RoomListPage(
          config: config,
          accessToken: 'tok',
          roomsRepository: fake,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Go live'));
    await tester.pumpAndSettle();

    expect(fake.createCalls, 1);
    expect(fake.startCalls, 1);
    expect(fake.publishCalls, 1);
    expect(find.text('You are live'), findsOneWidget);
    expect(find.textContaining('rtmp://localhost:1935/live/r-host'), findsOneWidget);
    expect(find.text('Copy push URL'), findsOneWidget);
    expect(find.text('Copy stream key'), findsOneWidget);
  });
}
