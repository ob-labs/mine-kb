# SeekDB 基础文档

> **版本**: SeekDB 0.0.1.dev4  
> **最后更新**: 2025-11-05

> **重要更新**: 从 0.0.1.dev4 版本开始，模块名称从 `oblite` 更改为 `seekdb`。详见 [升级指南](UPGRADE_SEEKDB_0.0.1.dev4.md)

# 1. 产品目标

轻量版嵌入式产品形态以库的形式集成在用户应用程序中，为开发者提供更强大灵活的数据管理解决方案，让数据管理无处不在(微控制器、物联网设备、边缘计算、移动应用、数据中心)，快速上手使用ALL IN ONE(TP、AP、 AI Native)的产品能力

![img](https://intranetproxy.alipay.com/skylark/lark/0/2025/png/275819/1756967663128-f86fe123-fc82-4e95-a87f-40c7e887eb04.png)

# 2. 安装配置

## 2.1 MineKB 应用自动安装

MineKB 应用会在启动时**自动检查并安装** SeekDB 依赖库（oblite.so）。

### 应用数据目录位置

- **macOS**: `~/Library/Application Support/com.mine-kb.app/`
- **Linux**: `~/.local/share/com.mine-kb.app/`
- **Windows**: `%APPDATA%\com.mine-kb.app\`

### 手动安装（可选）

如果自动下载失败，可以手动安装：

```bash
pip install seekdb==0.0.1.dev2 -i https://pypi.tuna.tsinghua.edu.cn/simple/
```

### 验证安装

查看应用日志，应该看到以下信息：

```
✅ oblite.so 存在，大小: XXXXX bytes
✅ PYTHONPATH 已配置
✅ SeekDB 数据库连接正常
✅ 应用启动成功！
```

## 2.2 独立使用 SeekDB

如果要在其他 Python 项目中使用 SeekDB：

**方法一：通过 pip 安装（推荐）**

```bash
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/
```

**方法二：直接下载（不推荐）**

```bash
# 注意：0.0.1.dev4 版本建议通过 pip 安装
# 直接下载 .so 文件的方式不再推荐
```

**最简使用（0.0.1.dev4 版本）**

```python
import seekdb
seekdb.open() # 默认打开本地数据库目录 oblite.db（可自定义路径）
conn = seekdb.connect() # 默认连接数据库 test
cursor = conn.cursor()
cursor.execute("create table t1(c1 int primary key, c2 int)")
```

> **注意**: 从 0.0.1.dev4 版本开始，使用 `import seekdb` 而非 `import oblite`。

# 3. AI Native

## 3.1 向量检索

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
cursor.execute("create table test_vector(c1 int primary key, c2 vector(2), vector index idx1(c2) with (distance=l2, type=hnsw, lib=vsag))")

cursor.execute("insert into test_vector values(1, [1, 1])")
cursor.execute("insert into test_vector values(2, [1, 2])")
cursor.execute("insert into test_vector values(3, [1, 3])")
conn.commit()

cursor.execute("SELECT c1 FROM test_vector ORDER BY l2_distance(c2, '[1, 2.5]') APPROXIMATE LIMIT 2;")
print(cursor.fetchall())
```

## 3.2 全文检索

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
sql='''create table articles (title VARCHAR(200) primary key, body Text, 
    FULLTEXT fts_idx(title, body));
    '''
cursor.execute(sql)

sql='''insert into articles(title, body) values
    ('OceanBase Tutorial', 'This is a tutorial about OceanBase Fulltext.'),
    ('Fulltext Index', 'Fulltext index can be very useful.'),
    ('OceanBase Test Case', 'Writing test cases helps ensure quality.')
    '''
cursor.execute(sql)
conn.commit()

sql='''select 
	title,
  match (title, body) against ("OceanBase") as score 
from
	articles
where
	match (title, body) against ("OceanBase")
order by
	score desc
    '''
cursor.execute(sql)
print(cursor.fetchall())
```

## 3.3 混合检索

待patch44x到轻量版功能

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
cursor.execute("create table doc_table(c1 int, vector vector(3), query varchar(255), content varchar(255), vector index idx1(vector) with (distance=l2, type=hnsw, lib=vsag), fulltext idx2(query), fulltext idx3(content))")

sql = '''insert into doc_table values(1, '[1,2,3]', "hello world", "oceanbase Elasticsearch database"),
                            (2, '[1,2,1]', "hello world, what is your name", "oceanbase mysql database"),
                            (3, '[1,1,1]', "hello world, how are you", "oceanbase oracle database"),
                            (4, '[1,3,1]', "real world, where are you from", "postgres oracle database"),
                            (5, '[1,3,2]', "real world, how old are you", "redis oracle database"),
                            (6, '[2,1,1]', "hello world, where are you from", "starrocks oceanbase database");'''
cursor.execute(sql)
conn.commit()

sql = '''set @parm = '{
      "query": {
        "bool": {
          "must": [
            {"match": {"query": "hi hello"}},
            {"match": { "content": "oceanbase mysql" }}
          ]
        }
      },
       "knn" : {
          "field": "vector",
          "k": 5,
          "num_candidates": 10,
          "query_vector": [1,2,3],
          "boost": 0.7
      },
      "_source" : ["query", "content", "_keyword_score", "_semantic_score"]
    }';'''
cursor.execute(sql)
sql = '''select dbms_hybrid_search.search('doc_table', @parm);'''
cursor.execute(sql)
print(cursor.fetchall())
```

# 4. 分析能力(OLAP)

## 4.1 数据导入

```bash
cat /data/1/example.csv
1,10
2,20
3,30
```



```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
cursor.execute("create table test_olap(c1 int, c2 int)")
cursor.execute("load data /*+ direct(true, 0) */ infile '/data/1/example.csv' into table test_olap fields terminated by ','")
cursor.execute("select count(*) from test_olap")
print(cursor.fetchall())
```

## 4.2 列存

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
sql='''create table each_column_group (col1 varchar(30) not null, col2 varchar(30) not null, col3 varchar(30) not null, col4 varchar(30) not null, col5 int) 
    with column group (each column);
    '''
cursor.execute(sql)
sql='''insert into each_column_group values('a', 'b', 'c', 'd', 1)
    '''
cursor.execute(sql)
conn.commit()
cursor.execute("select col1,col2 from each_column_group")
print(cursor.fetchall())
```

## 4.3 物化视图

```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
cursor.execute("create table base_t1(a int primary key, b int)")
cursor.execute("create table base_t2(c int primary key, d int)") 
cursor.execute("create materialized view log on base_t1 with(b)")
cursor.execute("create materialized view log on base_t2 with(d)") 
cursor.execute("create materialized view mv REFRESH fast START WITH sysdate() NEXT sysdate() + INTERVAL 1 second as select a,b,c,d from base_t1 join base_t2 on base_t1.a=base_t2.c")
cursor.execute("insert into base_t1 values(1, 10)")
cursor.execute("insert into base_t2 values(1, 100)")
conn.commit()

cursor.execute("select * from mv")
print(cursor.fetchall())
```

## 4.4 外表

```bash
cat /data/1/example.csv
1,10
2,20
3,30
```



```python
import seekdb

seekdb.open("./mine_kb.db")
conn = seekdb.connect("test")
cursor = conn.cursor()
sql='''CREATE EXTERNAL TABLE test_external_table(c1 int, c2 int) LOCATION='/data/1' FORMAT=(TYPE='CSV' FIELD_DELIMITER=',') PATTERN='example.csv';
'''
cursor.execute(sql)
cursor.execute("select * from test_external_table")
print(cursor.fetchall())
```

# 5. 事务能力(OLTP)

```python
import seekdb

# open db
seekdb.open("./mine_kb.db")
# get connect
conn = seekdb.connect("test")
# create table
cursor = conn.cursor()
cursor.execute("create table test_oltp(c1 int primary key, c2 int)")
# insert
cursor.execute("insert into test_oltp values(1, 10)")
cursor.execute("insert into test_oltp values(2, 20)")
cursor.execute("insert into test_oltp values(3, 30)")
conn.commit()
# select
cursor.execute("select *,ORA_ROWSCN from test_oltp")
print(cursor.fetchall())
```

**0.0.1.dev4 新特性：自动提交模式**

```python
import seekdb

seekdb.open("./mine_kb.db")
# 使用自动提交模式（无需手动 commit）
conn = seekdb.connect("test", autocommit=True)
cursor = conn.cursor()
cursor.execute("insert into test_oltp values(4, 40)")  # 自动提交
```

# 6. 平滑切换至分布式版本

用户通过嵌入式版本快速验证好产品原型后，想切换至分布式版本使用集群分布式处理能力，只需要修改导入包和相关配置即可，主体应用逻辑保持不变

```bash
import pymysql
conn = pymysql.connect(host='127.0.0.1', port=11002, user='root@sys', database='test')
```
