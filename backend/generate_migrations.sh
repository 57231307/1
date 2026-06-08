#!/bin/bash
cd /workspace/backend/migrations
mkdir -p ../migration/src
rm -f ../migration/src/m*.rs

count=1
mod_list=""

for dir in $(ls -d */ | sort); do
    dir=${dir%/}
    name="m$(printf "%04d" $count)_${dir#*_}"
    
    cat << EORUST > ../migration/src/${name}.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/${dir}/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/${dir}/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
EORUST

    mod_list="${mod_list}\npub mod ${name};"
    count=$((count + 1))
done

echo -e "$mod_list"
