from fakes import FakeServer
import pytest


@pytest.fixture
def fake_server():
    """Factory: fake_server({"/search": fn}) starts a stub server; all are stopped at teardown."""
    servers: list[FakeServer] = []

    def make(routes):
        server = FakeServer(routes)
        servers.append(server)
        return server

    yield make
    for server in servers:
        server.close()
