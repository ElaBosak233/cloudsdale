import { deleteUser, getUsers } from "@/api/user";
import UserCreateModal from "@/components/modals/admin/UserCreateModal";
import UserEditModal from "@/components/modals/admin/UserEditModal";
import MDIcon from "@/components/ui/MDIcon";
import { useConfigStore } from "@/stores/config";
import { User } from "@/types/user";
import {
    showErrNotification,
    showSuccessNotification,
} from "@/utils/notification";
import {
    ActionIcon,
    Avatar,
    Badge,
    Divider,
    Flex,
    Group,
    LoadingOverlay,
    Pagination,
    Paper,
    Select,
    Stack,
    Table,
    Text,
    TextInput,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { modals } from "@mantine/modals";
import { useEffect, useState } from "react";
import { Group as UGroup } from "@/types/user";

export default function Page() {
    const configStore = useConfigStore();

    const [refresh, setRefresh] = useState<number>(0);
    const [loading, setLoading] = useState<boolean>(false);

    const [users, setUsers] = useState<Array<User>>([]);
    const [page, setPage] = useState<number>(1);
    const [total, setTotal] = useState<number>(0);
    const [rowsPerPage, setRowsPerPage] = useState<number>(20);
    const [search, setSearch] = useState<string>("");
    const [searchInput, setSearchInput] = useState<string>("");
    const [sort, setSort] = useState<string>("id_asc");

    const [createOpened, { open: createOpen, close: createClose }] =
        useDisclosure(false);
    const [editOpened, { open: editOpen, close: editClose }] =
        useDisclosure(false);
    const [selectedUser, setSelectedUser] = useState<User>();

    function handleGetUsers() {
        setLoading(true);
        getUsers({
            page: page,
            size: rowsPerPage,
            name: search,
            sort_key: sort.split("_")[0],
            sort_order: sort.split("_")[1],
        })
            .then((res) => {
                const r = res.data;
                setUsers(r.data);
                setTotal(r.total);
            })
            .finally(() => {
                setLoading(false);
            });
    }

    function handleDeleteUser(user?: User) {
        deleteUser({
            id: Number(user?.id),
        })
            .then((_) => {
                showSuccessNotification({
                    message: `用户 ${user?.nickname} 已被删除`,
                });
            })
            .catch((e) => {
                showErrNotification({
                    message: e.response.data.message,
                });
            })
            .finally(() => {
                setRefresh((prev) => prev + 1);
            });
    }

    const openDeleteUserModal = (user?: User) =>
        modals.openConfirmModal({
            centered: true,
            children: (
                <>
                    <Flex gap={10} align={"center"}>
                        <MDIcon>person_remove</MDIcon>
                        <Text fw={600}>删除用户</Text>
                    </Flex>
                    <Divider my={10} />
                    <Text>你确定要删除用户 {user?.nickname} 吗？</Text>
                </>
            ),
            withCloseButton: false,
            labels: {
                confirm: "确定",
                cancel: "取消",
            },
            confirmProps: {
                color: "red",
            },
            onConfirm: () => {
                handleDeleteUser(user);
            },
        });

    useEffect(() => {
        handleGetUsers();
    }, [search, page, rowsPerPage, sort, refresh]);

    useEffect(() => {
        document.title = `用户管理 - ${configStore?.pltCfg?.site?.title}`;
    }, []);

    return (
        <>
            <Flex my={36} mx={"10%"} justify={"space-between"} gap={36}>
                <Stack w={"15%"} gap={0}>
                    <Flex justify={"space-between"} align={"center"}>
                        <TextInput
                            variant="filled"
                            placeholder="搜索"
                            mr={5}
                            value={searchInput}
                            onChange={(e) => setSearchInput(e.target.value)}
                            flex={1}
                        />
                        <ActionIcon
                            variant={"filled"}
                            onClick={() => setSearch(searchInput)}
                        >
                            <MDIcon size={15} c={"white"}>
                                search
                            </MDIcon>
                        </ActionIcon>
                    </Flex>
                    <Select
                        label="每页显示"
                        description="选择每页显示的用户数量"
                        value={String(rowsPerPage)}
                        allowDeselect={false}
                        data={["20", "25", "50", "100"]}
                        onChange={(_, option) =>
                            setRowsPerPage(Number(option.value))
                        }
                        mt={15}
                    />
                    <Select
                        label="排序"
                        description="选择用户排序方式"
                        value={sort}
                        allowDeselect={false}
                        data={[
                            {
                                label: "ID 升序",
                                value: "id_asc",
                            },
                            {
                                label: "ID 降序",
                                value: "id_desc",
                            },
                            {
                                label: "用户名升序",
                                value: "username_asc",
                            },
                            {
                                label: "用户名降序",
                                value: "username_desc",
                            },
                        ]}
                        onChange={(_, option) => setSort(option.value)}
                        mt={15}
                    />
                </Stack>
                <Stack
                    w={"85%"}
                    align="center"
                    gap={36}
                    mih={"calc(100vh - 10rem)"}
                >
                    <Paper w={"100%"} shadow={"md"} flex={1} pos={"relative"}>
                        <LoadingOverlay visible={loading} />
                        <Table stickyHeader horizontalSpacing={"md"} striped>
                            <Table.Thead>
                                <Table.Tr
                                    sx={{
                                        lineHeight: 3,
                                    }}
                                >
                                    <Table.Th>#</Table.Th>
                                    <Table.Th>用户名</Table.Th>
                                    <Table.Th>昵称</Table.Th>
                                    <Table.Th>电子邮箱</Table.Th>
                                    <Table.Th>权限组</Table.Th>
                                    <Table.Th>
                                        <Flex justify={"center"}>
                                            <ActionIcon onClick={createOpen}>
                                                <MDIcon>add</MDIcon>
                                            </ActionIcon>
                                        </Flex>
                                    </Table.Th>
                                </Table.Tr>
                            </Table.Thead>
                            <Table.Tbody>
                                {users?.map((user) => (
                                    <Table.Tr key={user?.id}>
                                        <Table.Th>
                                            <Badge>{user?.id}</Badge>
                                        </Table.Th>
                                        <Table.Th>
                                            <Group gap={15} wrap={"nowrap"}>
                                                <Avatar
                                                    color="brand"
                                                    src={`/api/users/${user?.id}/avatar`}
                                                    radius="xl"
                                                >
                                                    <MDIcon>person</MDIcon>
                                                </Avatar>
                                                <Text fw={700} size="1rem">
                                                    {user?.username}
                                                </Text>
                                            </Group>
                                        </Table.Th>
                                        <Table.Th>{user?.nickname}</Table.Th>
                                        <Table.Th>{user?.email}</Table.Th>
                                        <Table.Th>
                                            {user?.group === UGroup.Admin
                                                ? "管理员"
                                                : user?.group === UGroup.User
                                                  ? "普通用户"
                                                  : user?.group ===
                                                      UGroup.Banned
                                                    ? "封禁"
                                                    : "未知"}
                                        </Table.Th>
                                        <Table.Th>
                                            <Group
                                                justify="center"
                                                wrap={"nowrap"}
                                            >
                                                <ActionIcon
                                                    onClick={() => {
                                                        setSelectedUser(user);
                                                        editOpen();
                                                    }}
                                                >
                                                    <MDIcon>edit</MDIcon>
                                                </ActionIcon>
                                                <ActionIcon
                                                    onClick={() =>
                                                        openDeleteUserModal(
                                                            user
                                                        )
                                                    }
                                                >
                                                    <MDIcon color={"red"}>
                                                        delete
                                                    </MDIcon>
                                                </ActionIcon>
                                            </Group>
                                        </Table.Th>
                                    </Table.Tr>
                                ))}
                            </Table.Tbody>
                        </Table>
                    </Paper>
                    <Pagination
                        total={Math.max(Math.ceil(total / rowsPerPage), 1)}
                        value={page}
                        onChange={setPage}
                        withEdges
                    />
                </Stack>
            </Flex>
            <UserCreateModal
                setRefresh={() => {
                    setRefresh((prev) => prev + 1);
                }}
                opened={createOpened}
                onClose={createClose}
                centered
            />
            <UserEditModal
                setRefresh={() => {
                    setRefresh((prev) => prev + 1);
                }}
                userID={selectedUser?.id}
                opened={editOpened}
                onClose={editClose}
                centered
            />
        </>
    );
}
